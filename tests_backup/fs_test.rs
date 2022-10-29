// // Copyright 2021 Protocol Labs.
// //
// // Permission is hereby granted, free of charge, to any person obtaining a
// // copy of this software and associated documentation files (the "Software"),
// // to deal in the Software without restriction, including without limitation
// // the rights to use, copy, modify, merge, publish, distribute, sublicense,
// // and/or sell copies of the Software, and to permit persons to whom the
// // Software is furnished to do so, subject to the following conditions:
// //
// // The above copyright notice and this permission notice shall be included in
// // all copies or substantial portions of the Software.
// //
// // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// // OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// // FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// // DEALINGS IN THE SOFTWARE.
//
// //! # File sharing example
// //!
// //! Basic file sharing application with peers either providing or locating and
// //! getting files by name.
// //!
// //! While obviously showcasing how to build a basic file sharing application,
// //! the actual goal of this example is **to show how to integrate rust-libp2p
// //! into a larger application**.
// //!
// //! ## Sample plot
// //!
// //! Assuming there are 3 nodes, A, B and C. A and B each provide a file while C
// //! retrieves a file.
// //!
// //! Provider nodes A and B each provide a file, file FA and FB respectively.
// //! They do so by advertising themselves as a provider for their file on a DHT
// //! via [`libp2p-kad`]. The two, among other nodes of the network, are
// //! interconnected via the DHT.
// //!
// //! Node C can locate the providers for file FA or FB on the DHT via
// //! [`libp2p-kad`] without being connected to the specific node providing the
// //! file, but any node of the DHT. Node C then connects to the corresponding
// //! node and requests the file content of the file via
// //! [`libp2p-request-response`].
// //!
// //! ## Architectural properties
// //!
// //! - Clean clonable async/await interface ([`Client`]) to interact with the
// //!   network layer.
// //!
// //! - Single task driving the network layer, no locks required.
// //!
// //! ## Usage
// //!
// //! A two node setup with one node providing the file and one node requesting the file.
// //!
// //! 1. Run command below in one terminal.
// //!
// //!    ```
// /*
// cargo run --bin file-sharing -- \
// --listen-address /ip4/127.0.0.1/tcp/40837 \
// --secret-key-seed 1 \
// provide \
// --path trust_notes.md \
// --name a
//
// cargo run --bin file-sharing -- \
// --peer /ip4/127.0.0.1/tcp/40837/p2p/12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X \
// get \
// --name a
//
// */
// //!    ```
// //!
// //! 2. Run command below in another terminal.
// //!
// //!    ```
//
// //!    ```
// //!
// //! Note: The client does not need to be directly connected to the providing
// //! peer, as long as both are connected to some node on the same DHT.
//
// use async_std::io;
// use async_std::task::spawn;
// use futures::prelude::*;
// use libp2p::core::{Multiaddr, PeerId};
// use libp2p::multiaddr::Protocol;
// use std::error::Error;
// use std::path::PathBuf;
// use structopt::StructOpt;
// use futures::executor::block_on;
// use crate::network::{Client, EventLoop, Event};
// use futures::channel::mpsc::Receiver;
//
//
// struct P2P {
//     pub client: Client,
//     pub address: Multiaddr,
//     pub network_events: Receiver<Event>
// }
//
// impl P2P {
//
//     async fn event_process() {
//         loop {
//             match block_on(p2p.network_events.next()) {
//                 Some(network::Event::InternalEventAddress { multi_addr }) => {
//                     println!("omg we reached this here? {:?}", multi_addr);
//                     addr = Some(multi_addr);
//                 }
//                 // // Reply with the content of the file on incoming requests.
//                 // Some(network::Event::InboundRequest { request, channel }) => {
//                 //     if request == name {
//                 //         let file_content = std::fs::read_to_string(&path)?;
//                 //         network_client.respond_file(file_content, channel).await;
//                 //     }
//                 // }
//                 _ => {
//                     println!("wtf");
//                     // break;
//                 }
//             }
//         }
//         println!("omg we reached the end! here? {:?}", addr);
//     }
//
//     pub fn new(port: u16) -> P2P {
//         let network1 = network::new(None);
//         let networkk = block_on(network1).unwrap();
//         let (mut network_client, mut network_events, network_event_loop) =
//             networkk;
//         spawn(network_event_loop.run());
//         let address_str = "/ip4/0.0.0.0/tcp/".to_owned() + &port.to_string();
//         let address: Multiaddr = address_str.parse().unwrap();
//         let a2 = address.clone();
//         block_on(network_client.start_listening(address));
//         let p2p = P2P {
//             client: network_client,
//             address: a2,
//             network_events,
//         };
//         p2p
//     }
// }
//
// // https://users.rust-lang.org/t/should-i-use-async-or-use-a-separate-thread/22770/3
// #[test]
// fn testyer() -> Result<(), Box<dyn Error>> {
//     env_logger::init();
//
//     let mut p2p = P2P::new(4500);
//
//
//
//     let mut addr : Option<Multiaddr> = None;
//
//     match block_on(p2p.network_events.next()) {
//         Some(network::Event::InternalEventAddress { multi_addr }) => {
//             println!("omg we reached this here? {:?}", multi_addr);
//             addr = Some(multi_addr);
//         }
//         // // Reply with the content of the file on incoming requests.
//         // Some(network::Event::InboundRequest { request, channel }) => {
//         //     if request == name {
//         //         let file_content = std::fs::read_to_string(&path)?;
//         //         network_client.respond_file(file_content, channel).await;
//         //     }
//         // }
//         _ => {
//             println!("wtf");
//             // break;
//         }
//     }
//     println!("omg we reached the end! here? {:?}", addr);
//
//     Ok(())
// }
//
// #[derive(Debug, StructOpt)]
// #[structopt(name = "libp2p file sharing example")]
// struct Opt {
//     /// Fixed value to generate deterministic peer ID.
//     #[structopt(long)]
//     secret_key_seed: Option<u8>,
//
//     #[structopt(long)]
//     peer: Option<Multiaddr>,
//
//     #[structopt(long)]
//     listen_address: Option<Multiaddr>,
//
//     #[structopt(subcommand)]
//     argument: CliArgument,
// }
//
// #[derive(Debug, StructOpt)]
// enum CliArgument {
//     Provide {
//         #[structopt(long)]
//         path: PathBuf,
//         #[structopt(long)]
//         name: String,
//     },
//     Get {
//         #[structopt(long)]
//         name: String,
//     },
// }
//
// /// The network module, encapsulating all network related logic.
// mod network {
//     use super::*;
//     use async_trait::async_trait;
//     use futures::channel::{mpsc, oneshot};
//     use libp2p::core::either::EitherError;
//     use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
//     use libp2p::identity;
//     use libp2p::identity::ed25519;
//     use libp2p::kad::record::store::MemoryStore;
//     use libp2p::kad::{GetProvidersOk, Kademlia, KademliaEvent, QueryId, QueryResult};
//     use libp2p::multiaddr::Protocol;
//     use libp2p::request_response::{
//         ProtocolSupport, RequestId, RequestResponse, RequestResponseCodec, RequestResponseEvent,
//         RequestResponseMessage, ResponseChannel,
//     };
//     use libp2p::swarm::{ProtocolsHandlerUpgrErr, SwarmBuilder, SwarmEvent};
//     use libp2p::{NetworkBehaviour, Swarm};
//     use std::collections::{HashMap, HashSet};
//     use std::iter;
//     use futures::AsyncWriteExt;
//
//     /// Creates the network components, namely:
//     ///
//     /// - The network client to interact with the network layer from anywhere
//     ///   within your application.
//     ///
//     /// - The network event stream, e.g. for incoming requests.
//     ///
//     /// - The network task driving the network itself.
//     pub async fn new(
//         secret_key_seed: Option<u8>,
//     ) -> Result<(Client, Receiver<Event>, EventLoop), Box<dyn Error>> {
//         // Create a public/private key pair, either random or based on a seed.
//         let id_keys = match secret_key_seed {
//             Some(seed) => {
//                 let mut bytes = [0u8; 32];
//                 bytes[0] = seed;
//                 let secret_key = ed25519::SecretKey::from_bytes(&mut bytes).expect(
//                     "this returns `Err` only if the length is wrong; the length is correct; qed",
//                 );
//                 identity::Keypair::Ed25519(secret_key.into())
//             }
//             None => identity::Keypair::generate_ed25519(),
//         };
//         let peer_id = id_keys.public().into_peer_id();
//
//         // Build the Swarm, connecting the lower layer transport logic with the
//         // higher layer network behaviour logic.
//         let swarm = SwarmBuilder::new(
//             libp2p::development_transport(id_keys).await?,
//             ComposedBehaviour {
//                 kademlia: Kademlia::new(peer_id, MemoryStore::new(peer_id)),
//                 request_response: RequestResponse::new(
//                     FileExchangeCodec(),
//                     iter::once((FileExchangeProtocol(), ProtocolSupport::Full)),
//                     Default::default(),
//                 ),
//             },
//             peer_id,
//         )
//             .build();
//
//         let (command_sender, command_receiver) = mpsc::channel(0);
//         let (event_sender, event_receiver) = mpsc::channel(0);
//
//         Ok((
//             Client {
//                 sender: command_sender,
//             },
//             event_receiver,
//             EventLoop::new(swarm, command_receiver, event_sender),
//         ))
//     }
//
//     #[derive(Clone)]
//     pub struct Client {
//         sender: mpsc::Sender<Command>,
//     }
//
//     impl Client {
//         /// Listen for incoming connections on the given address.
//         pub async fn start_listening(
//             &mut self,
//             addr: Multiaddr,
//         ) -> Result<(), Box<dyn Error + Send>> {
//             let (sender, receiver) = oneshot::channel();
//             self.sender
//                 .send(Command::StartListening { addr, sender })
//                 .await
//                 .expect("Command receiver not to be dropped.");
//             receiver.await.expect("Sender not to be dropped.")
//         }
//
//         /// Dial the given peer at the given address.
//         pub async fn dial(
//             &mut self,
//             peer_id: PeerId,
//             peer_addr: Multiaddr,
//         ) -> Result<(), Box<dyn Error + Send>> {
//             let (sender, receiver) = oneshot::channel();
//             self.sender
//                 .send(Command::Dial {
//                     peer_id,
//                     peer_addr,
//                     sender,
//                 })
//                 .await
//                 .expect("Command receiver not to be dropped.");
//             receiver.await.expect("Sender not to be dropped.")
//         }
//
//         /// Advertise the local node as the provider of the given file on the DHT.
//         pub async fn start_providing(&mut self, file_name: String) {
//             let (sender, receiver) = oneshot::channel();
//             self.sender
//                 .send(Command::StartProviding { file_name, sender })
//                 .await
//                 .expect("Command receiver not to be dropped.");
//             receiver.await.expect("Sender not to be dropped.");
//         }
//
//         /// Find the providers for the given file on the DHT.
//         pub async fn get_providers(&mut self, file_name: String) -> HashSet<PeerId> {
//             let (sender, receiver) = oneshot::channel();
//             self.sender
//                 .send(Command::GetProviders { file_name, sender })
//                 .await
//                 .expect("Command receiver not to be dropped.");
//             receiver.await.expect("Sender not to be dropped.")
//         }
//
//         /// Request the content of the given file from the given peer.
//         pub async fn request_file(
//             &mut self,
//             peer: PeerId,
//             file_name: String,
//         ) -> Result<String, Box<dyn Error + Send>> {
//             let (sender, receiver) = oneshot::channel();
//             self.sender
//                 .send(Command::RequestFile {
//                     file_name,
//                     peer,
//                     sender,
//                 })
//                 .await
//                 .expect("Command receiver not to be dropped.");
//             receiver.await.expect("Sender not be dropped.")
//         }
//
//         /// Respond with the provided file content to the given request.
//         pub async fn respond_file(&mut self, file: String, channel: ResponseChannel<FileResponse>) {
//             self.sender
//                 .send(Command::RespondFile { file, channel })
//                 .await
//                 .expect("Command receiver not to be dropped.");
//         }
//     }
//
//     pub struct EventLoop {
//         swarm: Swarm<ComposedBehaviour>,
//         command_receiver: mpsc::Receiver<Command>,
//         event_sender: mpsc::Sender<Event>,
//         pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
//         pending_start_providing: HashMap<QueryId, oneshot::Sender<()>>,
//         pending_get_providers: HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>,
//         pending_request_file:
//         HashMap<RequestId, oneshot::Sender<Result<String, Box<dyn Error + Send>>>>,
//     }
//
//     impl EventLoop {
//         fn new(
//             swarm: Swarm<ComposedBehaviour>,
//             command_receiver: mpsc::Receiver<Command>,
//             event_sender: mpsc::Sender<Event>,
//         ) -> Self {
//             Self {
//                 swarm,
//                 command_receiver,
//                 event_sender,
//                 pending_dial: Default::default(),
//                 pending_start_providing: Default::default(),
//                 pending_get_providers: Default::default(),
//                 pending_request_file: Default::default(),
//             }
//         }
//
//         pub async fn run(mut self) {
//             loop {
//                 futures::select! {
//                     event = self.swarm.next() => self.handle_event(event.expect("Swarm stream to be infinite.")).await  ,
//                     command = self.command_receiver.next() => match command {
//                         Some(c) => self.handle_command(c).await,
//                         // Command channel closed, thus shutting down the network event loop.
//                         None=>  return,
//                     },
//                 }
//             }
//         }
//
//         async fn handle_event(
//             &mut self,
//             event: SwarmEvent<
//                 ComposedEvent,
//                 EitherError<ProtocolsHandlerUpgrErr<io::Error>, io::Error>,
//             >,
//         ) {
//             match event {
//                 SwarmEvent::Behaviour(ComposedEvent::Kademlia(
//                                           KademliaEvent::OutboundQueryCompleted {
//                                               id,
//                                               result: QueryResult::StartProviding(_),
//                                               ..
//                                           },
//                                       )) => {
//                     let sender: oneshot::Sender<()> = self
//                         .pending_start_providing
//                         .remove(&id)
//                         .expect("Completed query to be previously pending.");
//                     let _ = sender.send(());
//                 }
//                 SwarmEvent::Behaviour(ComposedEvent::Kademlia(
//                                           KademliaEvent::OutboundQueryCompleted {
//                                               id,
//                                               result: QueryResult::GetProviders(Ok(GetProvidersOk { providers, .. })),
//                                               ..
//                                           },
//                                       )) => {
//                     let _ = self
//                         .pending_get_providers
//                         .remove(&id)
//                         .expect("Completed query to be previously pending.")
//                         .send(providers);
//                 }
//                 SwarmEvent::Behaviour(ComposedEvent::Kademlia(_)) => {}
//                 SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
//                                           RequestResponseEvent::Message { message, .. },
//                                       )) => match message {
//                     RequestResponseMessage::Request {
//                         request, channel, ..
//                     } => {
//                         self.event_sender
//                             .send(Event::InboundRequest {
//                                 request: request.0,
//                                 channel,
//                             })
//                             .await
//                             .expect("Event receiver not to be dropped.");
//                     }
//                     RequestResponseMessage::Response {
//                         request_id,
//                         response,
//                     } => {
//                         let _ = self
//                             .pending_request_file
//                             .remove(&request_id)
//                             .expect("Request to still be pending.")
//                             .send(Ok(response.0));
//                     }
//                 },
//                 SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
//                                           RequestResponseEvent::OutboundFailure {
//                                               request_id, error, ..
//                                           },
//                                       )) => {
//                     let _ = self
//                         .pending_request_file
//                         .remove(&request_id)
//                         .expect("Request to still be pending.")
//                         .send(Err(Box::new(error)));
//                 }
//                 SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
//                                           RequestResponseEvent::ResponseSent { .. },
//                                       )) => {}
//                 SwarmEvent::NewListenAddr { address, .. } => {
//                     let local_peer_id = *self.swarm.local_peer_id();
//                     let addr2 = address.clone();
//                     println!(
//                         "Local node is listening on {:?}",
//                         address.with(Protocol::P2p(local_peer_id.into()))
//                     );
//                     self.event_sender.send(network::Event::InternalEventAddress {multi_addr: addr2})
//                         .await
//                         .expect("Event receiver not to be dropped.");;
//                 }
//                 SwarmEvent::IncomingConnection { .. } => {}
//                 SwarmEvent::ConnectionEstablished {
//                     peer_id, endpoint, ..
//                 } => {
//                     if endpoint.is_dialer() {
//                         if let Some(sender) = self.pending_dial.remove(&peer_id) {
//                             let _ = sender.send(Ok(()));
//                         }
//                     }
//                 }
//                 SwarmEvent::ConnectionClosed { .. } => {}
//                 SwarmEvent::UnreachableAddr {
//                     peer_id,
//                     attempts_remaining,
//                     error,
//                     ..
//                 } => {
//                     if attempts_remaining == 0 {
//                         if let Some(sender) = self.pending_dial.remove(&peer_id) {
//                             let _ = sender.send(Err(Box::new(error)));
//                         }
//                     }
//                 }
//                 e => panic!("{:?}", e),
//             }
//         }
//
//         async fn handle_command(&mut self, command: Command) {
//             match command {
//                 Command::StartListening { addr, sender } => {
//                     let _ = match self.swarm.listen_on(addr) {
//                         Ok(_) => sender.send(Ok(())),
//                         Err(e) => sender.send(Err(Box::new(e))),
//                     };
//                 }
//                 Command::Dial {
//                     peer_id,
//                     peer_addr,
//                     sender,
//                 } => {
//                     if self.pending_dial.contains_key(&peer_id) {
//                         todo!("Already dialing peer.");
//                     } else {
//                         self.swarm
//                             .behaviour_mut()
//                             .kademlia
//                             .add_address(&peer_id, peer_addr.clone());
//                         match self
//                             .swarm
//                             .dial_addr(peer_addr.with(Protocol::P2p(peer_id.into())))
//                         {
//                             Ok(()) => {
//                                 self.pending_dial.insert(peer_id, sender);
//                             }
//                             Err(e) => {
//                                 let _ = sender.send(Err(Box::new(e)));
//                             }
//                         }
//                     }
//                 }
//                 Command::StartProviding { file_name, sender } => {
//                     let query_id = self
//                         .swarm
//                         .behaviour_mut()
//                         .kademlia
//                         .start_providing(file_name.into_bytes().into())
//                         .expect("No store error.");
//                     self.pending_start_providing.insert(query_id, sender);
//                 }
//                 Command::GetProviders { file_name, sender } => {
//                     let query_id = self
//                         .swarm
//                         .behaviour_mut()
//                         .kademlia
//                         .get_providers(file_name.into_bytes().into());
//                     self.pending_get_providers.insert(query_id, sender);
//                 }
//                 Command::RequestFile {
//                     file_name,
//                     peer,
//                     sender,
//                 } => {
//                     let request_id = self
//                         .swarm
//                         .behaviour_mut()
//                         .request_response
//                         .send_request(&peer, FileRequest(file_name));
//                     self.pending_request_file.insert(request_id, sender);
//                 }
//                 Command::RespondFile { file, channel } => {
//                     self.swarm
//                         .behaviour_mut()
//                         .request_response
//                         .send_response(channel, FileResponse(file))
//                         .expect("Connection to peer to be still open.");
//                 }
//             }
//         }
//     }
//
//     #[derive(NetworkBehaviour)]
//     #[behaviour(event_process = false, out_event = "ComposedEvent")]
//     struct ComposedBehaviour {
//         request_response: RequestResponse<FileExchangeCodec>,
//         kademlia: Kademlia<MemoryStore>,
//     }
//
//     #[derive(Debug)]
//     enum ComposedEvent {
//         RequestResponse(RequestResponseEvent<FileRequest, FileResponse>),
//         Kademlia(KademliaEvent),
//     }
//
//     impl From<RequestResponseEvent<FileRequest, FileResponse>> for ComposedEvent {
//         fn from(event: RequestResponseEvent<FileRequest, FileResponse>) -> Self {
//             ComposedEvent::RequestResponse(event)
//         }
//     }
//
//     impl From<KademliaEvent> for ComposedEvent {
//         fn from(event: KademliaEvent) -> Self {
//             ComposedEvent::Kademlia(event)
//         }
//     }
//
//     #[derive(Debug)]
//     enum Command {
//         StartListening {
//             addr: Multiaddr,
//             sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
//         },
//         Dial {
//             peer_id: PeerId,
//             peer_addr: Multiaddr,
//             sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
//         },
//         StartProviding {
//             file_name: String,
//             sender: oneshot::Sender<()>,
//         },
//         GetProviders {
//             file_name: String,
//             sender: oneshot::Sender<HashSet<PeerId>>,
//         },
//         RequestFile {
//             file_name: String,
//             peer: PeerId,
//             sender: oneshot::Sender<Result<String, Box<dyn Error + Send>>>,
//         },
//         RespondFile {
//             file: String,
//             channel: ResponseChannel<FileResponse>,
//         },
//     }
//
//     pub enum Event {
//         InboundRequest {
//             request: String,
//             channel: ResponseChannel<FileResponse>,
//         },
//         InboundTransaction {
//             request: Transaction,
//             channel: ResponseChannel<FileResponse>,
//         },
//         InternalEventAddress {
//             multi_addr: Multiaddr
//         },
//     }
//
//     // Simple file exchange protocol
//
//     #[derive(Debug, Clone)]
//     struct FileExchangeProtocol();
//     #[derive(Clone)]
//     struct FileExchangeCodec();
//     #[derive(Debug, Clone, PartialEq, Eq)]
//     struct FileRequest(String);
//     #[derive(Debug, Clone, PartialEq, Eq)]
//     pub struct FileResponse(String);
//
//     impl ProtocolName for FileExchangeProtocol {
//         fn protocol_name(&self) -> &[u8] {
//             "/file-exchange/1".as_bytes()
//         }
//     }
//
//     #[async_trait]
//     impl RequestResponseCodec for FileExchangeCodec {
//         type Protocol = FileExchangeProtocol;
//         type Request = FileRequest;
//         type Response = FileResponse;
//
//         async fn read_request<T>(
//             &mut self,
//             _: &FileExchangeProtocol,
//             io: &mut T,
//         ) -> io::Result<Self::Request>
//             where
//                 T: AsyncRead + Unpin + Send,
//         {
//             let vec = read_length_prefixed(io, 1_000_000).await?;
//
//             if vec.is_empty() {
//                 return Err(io::ErrorKind::UnexpectedEof.into());
//             }
//
//             Ok(FileRequest(String::from_utf8(vec).unwrap()))
//         }
//
//         async fn read_response<T>(
//             &mut self,
//             _: &FileExchangeProtocol,
//             io: &mut T,
//         ) -> io::Result<Self::Response>
//             where
//                 T: AsyncRead + Unpin + Send,
//         {
//             let vec = read_length_prefixed(io, 1_000_000).await?;
//
//             if vec.is_empty() {
//                 return Err(io::ErrorKind::UnexpectedEof.into());
//             }
//
//             Ok(FileResponse(String::from_utf8(vec).unwrap()))
//         }
//
//         async fn write_request<T>(
//             &mut self,
//             _: &FileExchangeProtocol,
//             io: &mut T,
//             FileRequest(data): FileRequest,
//         ) -> io::Result<()>
//             where
//                 T: AsyncWrite + Unpin + Send,
//         {
//             write_length_prefixed(io, data).await?;
//             io.close().await?;
//
//             Ok(())
//         }
//
//         async fn write_response<T>(
//             &mut self,
//             _: &FileExchangeProtocol,
//             io: &mut T,
//             FileResponse(data): FileResponse,
//         ) -> io::Result<()>
//             where
//                 T: AsyncWrite + Unpin + Send,
//         {
//             write_length_prefixed(io, data).await?;
//             io.close().await?;
//
//             Ok(())
//         }
//     }
// }
