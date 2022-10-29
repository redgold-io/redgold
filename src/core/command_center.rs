#[derive(Debug)]
pub enum Command {
    // DoNothing,
    // StartListening {
    //     addr: Multiaddr,
    //     sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    // },
    // Dial {
    //     peer_id: PeerId,
    //     peer_addr: Multiaddr,
    //     sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    // },
    // // StartProviding {
    // //     file_name: String,
    // //     sender: oneshot::Sender<()>,
    // // },
    // // GetProviders {
    // //     file_name: String,
    // //     sender: oneshot::Sender<HashSet<PeerId>>,
    // // },
    // SendRequest {
    //     request: Request,
    //     peer: PeerId,
    //     sender: oneshot::Sender<Result<Response, Box<dyn Error + Send>>>,
    // },
    // Respond {
    //     response: Response,
    //     channel: ResponseChannel<PeerResponse>,
    // },
}

struct RunContext {}

pub async fn run_loop() {}
