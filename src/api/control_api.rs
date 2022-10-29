use bitcoin::secp256k1::PublicKey;
use std::sync::Arc;
// use futures::channel::mpsc;
use futures::executor::block_on;
// use futures::{SinkExt, StreamExt};
use log::{error, info};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use warp::Filter;
use redgold_schema::structs::ErrorInfo;

use crate::util::to_libp2p_peer_id;

use crate::core::relay::Relay;
use crate::schema::structs::{
    AddPeerFullRequest, ControlRequest, ControlResponse, ResponseMetadata,
};

// https://github.com/rustls/hyper-rustls/blob/master/examples/server.rs
#[allow(dead_code)]
#[derive(Clone)]
pub struct ControlClient {
    url: String,
    port: u16,
}

impl ControlClient {
    #[allow(dead_code)]
    pub fn default() -> Self {
        Self {
            url: "localhost".to_string(),
            port: 6060,
        }
    }

    pub fn local(port_offset: u16) -> Self {
        Self {
            url: "localhost".to_string(),
            port: port_offset,
        }
    }
    #[allow(dead_code)]
    fn formatted_url(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*self.port.to_string();
    }
    #[allow(dead_code)]
    pub async fn request(
        &self,
        a: &ControlRequest,
    ) -> Result<ControlResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let res = client
            .post(self.formatted_url() + "/control")
            .json(&a)
            .send()
            .await?
            .json::<ControlResponse>()
            .await?;
        Ok(res)
    }
}

#[derive(Clone)]
pub struct ControlServer {
    pub relay: Relay,
    //control_channel: Channel<ControlRequest>
    // pub p2p_client: crate::api::p2p_io::rgnetwork::Client,
    pub runtime: Arc<Runtime>,
}

impl ControlServer {
    async fn run_control_server(self) -> Result<(), ErrorInfo> {
        let Self {
            relay,
            // p2p_client,
            runtime,
        } = self;
        let relay2 = relay.clone();
        info!(
            "Starting control server on port: {:?}",
            relay.node_config.control_port()
        );
        let add_peer = warp::post()
            .and(warp::path("control"))
            // Only accept bodies smaller than 16kb...
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json::<ControlRequest>())
            .map(move |req: ControlRequest| {
                metrics::increment_counter!("redgold.api.control.num_requests");
                info!("Control request received");

                // let mut p2p_client2 = p2p_client.clone();
                let mut success = true;
                let ControlRequest {
                    add_peer_full_request,
                } = req;
                if add_peer_full_request.is_some() {
                    let add: AddPeerFullRequest = add_peer_full_request.unwrap();
                    let res = relay.ds.insert_peer_single(
                        &add.id,
                        add.trust,
                        &add.public_key,
                        add.address.clone(),
                    );
                    success = res.is_ok();
                    if res.is_ok() {
                        if add.connect_to_peer {
                            info!("Dialing address: {}", add.address.clone());
                            // block_on(p2p_client2.dial(
                            //     to_libp2p_peer_id(
                            //         &PublicKey::from_slice(&*add.public_key).unwrap(),
                            //     ),
                            //     add.address.parse().unwrap(),
                            // ))
                            // .expect("done");
                        }
                    } else {
                        error!("Error {}", res.err().unwrap());
                    }
                }
                warp::reply::json(&ControlResponse {
                    response_metadata: Some(ResponseMetadata {
                        success,
                        error_info: None,
                    }),
                })
            });

        warp::serve(add_peer)
            .run(([127, 0, 0, 1], relay2.node_config.control_port()))
            .await;
        Ok(())
    }
    pub fn start(self) -> JoinHandle<Result<(), ErrorInfo>> {
        return self.runtime.clone().spawn(self.run_control_server());
    }
}
//
// #[tokio::test]
// async fn test_warp_control_basic() {
//     crate::util::init_logger();
//     println!("WTF");
//     let tc = TestConstants::new();
//     let store = DataStore::in_memory();
//     let c = store.create_all();
//     let (mut command_sender, mut command_receiver) = mpsc::channel(0);
//     let p2p_client = crate::p2p_io::rgnetwork::Client {
//         sender: command_sender.clone(),
//     };
//
//     println!("before spawn");
//     let mut c2 = command_receiver;
//     tokio::spawn(async move {
//         loop {
//             let event = c2.next().await;
//             println!("Event received: {:?}", event);
//         }
//     });
//
//     command_sender.clone().send(Command::DoNothing {}).await;
//     println!("after spawn");
//     let cs = ControlServer {
//         ds: store.clone(),
//         port: 6060,
//         p2p_client,
//     };
//     cs.start();
//     sleep(Duration::new(1, 0));
//     let expected_trust = 1.0;
//     let res = ControlClient::default()
//         .request(&ControlRequest {
//             add_peer_full: Some(AddPeerFull {
//                 id: tc.public_peer_id.clone(),
//                 trust: expected_trust,
//                 public_key: tc.public.serialize().to_vec(),
//                 address: "/ip4/127.0.0.1/tcp/54613".to_string(),
//             }),
//         })
//         .await
//         .unwrap();
//     let vec2 = tc.public_peer_id.clone();
//     let trust = store.clone().select_peer_trust(&vec![vec2]).unwrap();
//     let vec1 = tc.public_peer_id.clone();
//     let trust_act = trust.get(&vec1).unwrap();
//     assert_eq!(&expected_trust, trust_act);
//     assert_eq!(
//         Response {
//             success: true,
//             error_code: None
//         },
//         res
//     );
// }
