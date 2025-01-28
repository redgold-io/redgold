use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use async_lsp::{LanguageClient, LanguageServer, ResponseError, ServerSocket};
use async_process::{ChildStdin, ChildStdout};
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;
use lsp_types::{ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, HoverContents, HoverParams, InitializeParams, InitializedParams, MarkupContent, NumberOrString, PartialResultParams, Position, ProgressParams, ProgressParamsValue, PublishDiagnosticsParams, ShowMessageParams, SymbolKind, TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url, WindowClientCapabilities, WorkDoneProgress, WorkDoneProgressParams};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tracing::{info, Level};


#[derive(Clone)]
struct ClientState {
    indexed_tx: Option<flume::Sender<()>>,
}

impl LanguageClient for ClientState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn progress(&mut self, params: ProgressParams) -> Self::NotifyResult {
        tracing::info!("{:?} {:?}", params.token, params.value);
        if matches!(params.token, NumberOrString::String(s) if s == "rustAnalyzer/Indexing")
            && matches!(
                params.value,
                ProgressParamsValue::WorkDone(WorkDoneProgress::End(_))
            )
        {
            // Sometimes rust-analyzer auto-index multiple times?
            if let Some(tx) = self.indexed_tx.take() {
                let _: Result<_, _> = tx.send(());
            }
        }
        ControlFlow::Continue(())
    }

    fn publish_diagnostics(&mut self, _: PublishDiagnosticsParams) -> Self::NotifyResult {
        ControlFlow::Continue(())
    }

    fn show_message(&mut self, params: ShowMessageParams) -> Self::NotifyResult {
        tracing::info!("Message {:?}: {}", params.typ, params.message);
        ControlFlow::Continue(())
    }
}

impl ClientState {
    fn new_router(indexed_tx: flume::Sender<()>) -> Router<Self> {
        let mut router = Router::from_language_client(ClientState {
            indexed_tx: Some(indexed_tx),
        });
        router.event(Self::on_stop);
        router
    }

    fn on_stop(&mut self, _: Stop) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Break(Ok(()))
    }
}

struct Stop;


struct ServerWrapper {
    server: ServerSocket,
    root_dir: PathBuf,
    mainloop_fut: JoinHandle<()>,
}

impl ServerWrapper {

    async fn hover(&mut self, file_uri: Url, text: &str) {
// Query.
        let var_pos = text.find("var").unwrap();
        let hover = self.server
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: file_uri },
                    position: Position::new(0, var_pos as _),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .unwrap()
            .unwrap();
        info!("Hover result: {hover:?}");
        assert!(
            matches!(
            hover.contents,
            HoverContents::Markup(MarkupContent { value, .. })
            if value.contains("let var: i32")
        ),
            "should show the type of `var`",
        );
    }

    async fn find_functions(&mut self, file_uri: Url) -> Vec<String> {
        let symbols = self.server
            .document_symbol(DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: file_uri },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .unwrap();

        let mut functions = Vec::new();
        if let Some(symbols) = symbols {
            match symbols {
                DocumentSymbolResponse::Flat(flat_symbols) => {
                    for symbol in flat_symbols {
                        if let SymbolKind::FUNCTION = symbol.kind {
                            functions.push(symbol.name);
                        }
                    }
                }
                DocumentSymbolResponse::Nested(nested_symbols) => {
                    fn process_nested_symbols(symbols: &[DocumentSymbol], functions: &mut Vec<String>) {
                        for symbol in symbols {
                            if let SymbolKind::FUNCTION = symbol.kind {
                                functions.push(symbol.name.clone());
                            }
                            if let Some(c) = symbol.children.as_ref() {
                                process_nested_symbols(c, functions);
                            }
                        }
                    }

                    process_nested_symbols(&nested_symbols, &mut functions);
                }
            }
        }
        functions
    }
    async fn shutdown(mut self) {
        // Shutdown.
        self.server.shutdown(()).await.unwrap();
        self.server.exit(()).unwrap();

        self.server.emit(Stop).unwrap();
        self.mainloop_fut.await.unwrap();
    }

}

#[tokio::test(flavor = "current_thread")]
async fn working_test2() {
    let root_dir = init_rootdir();

    let (indexed_rx, mut server, mainloop_fut) = server_all(&root_dir);

    let mut server2 = server.clone();
    let mut server3 = server.clone();
    let mut server4 = server.clone();
    init_server(&root_dir, &mut server2).await;

    let (file_uri, text) = sync(root_dir.clone(), indexed_rx, &mut server3).await;

    let mut sw = ServerWrapper {
        server: server4,
        root_dir: root_dir.clone(),
        mainloop_fut,
    };

    sw.hover(file_uri, text).await;

    let file_path = root_dir.join("src/directory_code_reader.rs");
    let file_uri = Url::from_file_path(file_path).unwrap();
    let functions = sw.find_functions(file_uri).await;
    info!("Functions defined in the file: {:?}", functions);
    sw.shutdown().await;

    // hover(&mut server, file_uri, text).await;


}

async fn sync(root_dir: PathBuf, indexed_rx: flume::Receiver<()>, server: &mut ServerSocket) -> (Url, &str) {
// Synchronize documents.
    let file_uri = Url::from_file_path(root_dir.join("src/lib.rs")).unwrap();
    let text = "#![no_std] fn func() { let var = 1; }";
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: file_uri.clone(),
                language_id: "rust".into(),
                version: 0,
                text: text.into(),
            },
        })
        .unwrap();

    // Wait until indexed.
    indexed_rx.recv_async().await.unwrap();
    (file_uri, text)
}

async fn init_server(root_dir: &PathBuf, server: &mut ServerSocket) {
// Initialize.
    let root_uri = Url::from_file_path(&root_dir).unwrap();
    let init_ret = server
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            capabilities: ClientCapabilities {
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    ..WindowClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        })
        .await
        .unwrap();
    info!("Initialized: {init_ret:?}");
    server.initialized(InitializedParams {}).unwrap();
}

fn server_all(root_dir: &PathBuf) -> (flume::Receiver<()>, ServerSocket, JoinHandle<()>) {
    let (indexed_tx, indexed_rx) = flume::unbounded();
    let (mainloop, mut server) = async_lsp::MainLoop::new_client(|_server| {
        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .service(ClientState::new_router(indexed_tx))
    });

    let (stdout, stdin) = bg_rust_analyzer(&root_dir);

    let mainloop_fut = tokio::spawn(async move {
        mainloop.run_buffered(stdout, stdin).await.unwrap();
    });
    (indexed_rx, server, mainloop_fut)
}

fn bg_rust_analyzer(root_dir: &PathBuf) -> (ChildStdout, ChildStdin) {
    let child = async_process::Command::new("rust-analyzer")
        .current_dir(&root_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(false)
        .spawn()
        .expect("Failed run rust-analyzer");
    let stdout = child.stdout.unwrap();
    let stdin = child.stdin.unwrap();
    (stdout, stdin)
}

fn init_rootdir() -> PathBuf {
    let root_dir = Path::new(".")
        .canonicalize()
        .expect("test root should be valid");

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();
    root_dir
}
