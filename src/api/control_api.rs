use redgold_common::client::http::RgHttpClient;
use crate::api::rosetta::models::Error;
use crate::api::warp_helpers::as_warp_json_response;
use bdk::bitcoin::secp256k1::PublicKey;
// use futures::channel::mpsc;
use futures::executor::block_on;
use itertools::Itertools;
use redgold_schema::helpers::easy_json::json_or;
use redgold_schema::structs::{BytesData, ControlMultipartyKeygenRequest, ControlMultipartyKeygenResponse, ControlMultipartySigningRequest, ControlMultipartySigningResponse, ErrorInfo, InitiateMultipartyKeygenRequest, InitiateMultipartyKeygenResponse, InitiateMultipartySigningRequest, InitiateMultipartySigningResponse, MultipartyIdentifier};
use redgold_schema::message::Request;
use redgold_schema::{response_metadata, structs, RgResult, SafeOption};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tracing::trace;
// use futures::{SinkExt, StreamExt};
use tracing::{error, info};
use uuid::Uuid;
use warp::reply::Json;
use warp::{Filter, Rejection};

// use crate::util::to_libp2p_peer_id;

use crate::core::relay::Relay;
use crate::schema::structs::{
    ControlRequest, ControlResponse, ResponseMetadata,
};

// https://github.com/rustls/hyper-rustls/blob/master/examples/server.rs
#[allow(dead_code)]
#[derive(Clone)]
pub struct ControlClient {
    client: RgHttpClient
}

impl ControlClient {

    pub async fn request(&self, cr: ControlRequest) -> Result<ControlResponse, ErrorInfo> {
        let result: ControlResponse = self.client.json_post(&cr, "control".into()).await?;
        result.as_error_info()?;
        Ok(result)
    }

    pub async fn multiparty_keygen(&self, req: Option<ControlMultipartyKeygenRequest>)
        -> RgResult<ControlMultipartyKeygenResponse> {
        let mut cr = ControlRequest::empty();
        let req = req.unwrap_or(ControlMultipartyKeygenRequest::default());
        cr.control_multiparty_keygen_request = Some(req);

        info!("Sending multiparty control request");
        let res: ControlResponse = self.request(cr).await?;

        res.control_multiparty_keygen_response.ok_or(ErrorInfo::error_info("No response"))
    }

    pub async fn multiparty_signing(&self,
                                    req: ControlMultipartySigningRequest,
    ) -> RgResult<ControlMultipartySigningResponse> {
        let mut cr = ControlRequest::empty();
        cr.control_multiparty_signing_request = Some(req);
        info!("Sending multiparty signing control request");
        let res: ControlResponse = self.request(cr).await?;
        res.control_multiparty_signing_response.ok_or(ErrorInfo::error_info("No response"))
    }

    pub fn local(port: u16) -> Self {
        Self {
            client: RgHttpClient::new("localhost".to_string(), port, None)
        }
    }

    pub fn new(client: RgHttpClient) -> Self {
        Self {
            client
        }
    }

}

#[derive(Clone)]
pub struct ControlServer {
    pub relay: Relay,
    //control_channel: Channel<ControlRequest>
    // pub p2p_client: crate::api::p2p_io::rgnetwork::Client,
    // pub runtime: Arc<Runtime>,
}

impl ControlServer {

    async fn request_response(request: ControlRequest, relay: Relay
                              // , rt: Arc<Runtime>
    )-> Result<ControlResponse, ErrorInfo> {
        metrics::counter!("redgold.api.control.num_requests").increment(1);
        info!("Control request received");
        let response = ControlResponse::empty();
        Ok(response)
    }

    async fn run_control_server(self) -> Result<(), ErrorInfo> {
        let Self {
            relay,
            // p2p_client,
            // runtime,
        } = self;
        let relay2 = relay.clone();
        trace!(
            "Starting control server on port: {:?}",
            relay.node_config.control_port()
        );
        // let rt2 = runtime.clone();
        let control_relay = relay.clone();
        let control_single_json = warp::post()
            .and(warp::path("control"))
            // Only accept bodies smaller than 16kb...
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json::<ControlRequest>())
            .and_then(move |req: ControlRequest| {
                // let rt3 = rt2.clone();
                let rl2 = control_relay.clone();
                async move {
                    let relay_int = rl2.clone();
                    Self::handle_control(req, relay_int,
                                         // rt3
                    ).await
                }
            });

        warp::serve(control_single_json)
            .run(([127, 0, 0, 1], relay2.node_config.control_port()))
            .await;
        Ok(())
    }

    async fn handle_control(req: ControlRequest, relay_int: Relay
                            // , rt: Arc<Runtime>
    ) -> Result<Json, Rejection> {
        let response =
            Self::request_response(req, relay_int.clone()
                                   // , rt.clone()
            ).await;
        let res = response.map_err(|e| {
            let mut r = ControlResponse::empty();
            r.response_metadata = Some(e.response_metadata());
            r
        });
        as_warp_json_response(res)
    }
    pub fn start(self) -> JoinHandle<Result<(), ErrorInfo>> { return tokio::spawn(self.run_control_server())
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
