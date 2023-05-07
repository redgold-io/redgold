use std::collections::HashMap;
use bitcoin::secp256k1::PublicKey;
use std::sync::Arc;
use std::time::Duration;
// use futures::channel::mpsc;
use futures::executor::block_on;
use itertools::Itertools;
// use futures::{SinkExt, StreamExt};
use log::{error, info};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use uuid::Uuid;
use warp::{Filter, Rejection};
use warp::reply::Json;
use redgold_schema::{json_or, response_metadata, SafeOption, structs};
use redgold_schema::structs::{BytesData, ErrorInfo, InitiateMultipartyKeygenRequest, InitiateMultipartyKeygenResponse, InitiateMultipartySigningRequest, InitiateMultipartySigningResponse, MultipartyIdentifier, Request};
use crate::api::{as_warp_json_response, HTTPClient};
use crate::api::rosetta::models::Error;

use crate::util::to_libp2p_peer_id;

use crate::core::relay::Relay;
use crate::multiparty::initiate_mp::{initiate_mp_keygen, initiate_mp_keysign};
use crate::schema::structs::{
    AddPeerFullRequest, ControlRequest, ControlResponse, ResponseMetadata,
};

// https://github.com/rustls/hyper-rustls/blob/master/examples/server.rs
#[allow(dead_code)]
#[derive(Clone)]
pub struct ControlClient {
    client: HTTPClient
}

impl ControlClient {

    pub async fn request(&self, cr: ControlRequest) -> Result<ControlResponse, ErrorInfo> {
        let result: ControlResponse = self.client.json_post(&cr, "control".into()).await?;
        result.as_error_info()?;
        Ok(result)
    }

    pub async fn multiparty_keygen(&self, req: Option<InitiateMultipartyKeygenRequest>) -> Result<InitiateMultipartyKeygenResponse, ErrorInfo> {
        let mut cr = ControlRequest::empty();
        let req = req.unwrap_or(InitiateMultipartyKeygenRequest::default());
        cr.initiate_multiparty_keygen_request = Some(req);

        info!("Sending multiparty control request");
        let res: ControlResponse = self.request(cr).await?;

        res.initiate_multiparty_keygen_response.ok_or(ErrorInfo::error_info("No response"))
    }

    pub async fn multiparty_signing(&self,
                                    req: Option<InitiateMultipartySigningRequest>,
                                    keygen: Option<InitiateMultipartyKeygenRequest>,
                                    data_to_sign: BytesData,
    ) -> Result<InitiateMultipartySigningResponse, ErrorInfo> {
        let mut cr = ControlRequest::empty();
        let mut req = req.unwrap_or(InitiateMultipartySigningRequest::default());
        req.keygen_room = keygen;
        req.data_to_sign = Some(data_to_sign);
        cr.initiate_multiparty_signing_request = Some(req);

        info!("Sending multiparty signing control request");
        let res: ControlResponse = self.request(cr).await?;
        res.initiate_multiparty_signing_response.ok_or(ErrorInfo::error_info("No response"))
    }

    pub fn local(port: u16) -> Self {
        Self {
            client: HTTPClient::new("localhost".to_string(), port)
        }
    }

    pub fn new(client: HTTPClient) -> Self {
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
    pub runtime: Arc<Runtime>,
}

impl ControlServer {

    async fn find_multiparty_key_pairs(relay: Relay) -> Result<Vec<structs::PublicKey>, ErrorInfo> {

        let peers = relay.ds.peer_store.all_peers().await?;
        // TODO: Safer, query all pk
        let pk =
            peers.iter().map(|p| p.node_metadata.get(0).clone().unwrap().public_key.clone().unwrap())
                .collect_vec();

        info!("Mulitparty found {} possible peers", pk.len());
        let results = relay.broadcast(
            pk, Request::empty().about(), Some(Duration::from_secs(20))).await;
        let valid_pks = results.iter()
            .filter_map(|(pk, r)| if r.is_ok() { Some(pk.clone()) } else { None })
            .collect_vec();
        info!("Mulitparty found {} valid_pks peers", valid_pks.len());
        if valid_pks.len() == 0 {
            return Err(ErrorInfo::error_info("No valid peers found"));
        }
        let mut keys = vec![relay.node_config.public_key()];
        keys.extend(valid_pks.clone());
        Ok(keys)
    }


    fn fill_identifier(keys: Vec<structs::PublicKey>, identifier: Option<MultipartyIdentifier>) -> Option<MultipartyIdentifier> {
        let num_parties = keys.len() as i64;
        if let Some(ident) = identifier {
            let mut identifier = ident;
            identifier.uuid = Uuid::new_v4().to_string();
            identifier.party_keys = if identifier.party_keys.is_empty() {
                keys.clone()
            } else {
                identifier.party_keys
            };
            Some(identifier)
        } else {
            let mut threshold: i64 = (num_parties / 2) as i64;
            if threshold < 1 {
                threshold = 1;
            }
            if threshold > (num_parties - 1) {
                threshold = num_parties - 1;
            }
            Some(
                MultipartyIdentifier {
                    party_keys: keys.clone(),
                    threshold,
                    uuid: Uuid::new_v4().to_string(),
                    num_parties
                }
            )
        }
    }

    async fn request_response(request: ControlRequest, relay: Relay, rt: Arc<Runtime>)-> Result<ControlResponse, ErrorInfo> {
        metrics::increment_counter!("redgold.api.control.num_requests");
        info!("Control request received");

        let mut response = ControlResponse::empty();

        // TODO: Shouldn't both of these really be in the initiate function?
        if let Some(mps) = request.initiate_multiparty_keygen_request {

            let keys = Self::find_multiparty_key_pairs(relay.clone()).await?;
            let num_parties = keys.len() as i64;

            let mut req = mps.clone();
            // TODO: Separate request types so we don't have to do this
            req.index = Some(1);
            req.port = Some(relay.node_config.mparty_port().clone() as u32);
            req.host_address = Some(relay.node_config.external_ip.clone()); // Some("127.0.0.1".to_string());
            req.store_local_share = Some(true);
            req.return_local_share = Some(true);
            req.host_key = Some(relay.node_config.public_key().clone());
            req.identifier = Self::fill_identifier(keys, req.identifier);
            // let d = mps.clone();
            info!("Initiate multiparty request: {}", json_or(&req));
            let result = initiate_mp_keygen(relay.clone(), req, rt.clone()).await?;
            response.initiate_multiparty_keygen_response = Some(result);
        } else if let Some(mut req) = request.initiate_multiparty_signing_request {
            let keygen = req.keygen_room.safe_get()?;
            let keys = Self::find_multiparty_key_pairs(relay.clone()).await?;
            req.identifier = Self::fill_identifier(keys, req.identifier);
            // TODO: Remove these from schema, enforce node knowing about this
            req.port = Some(relay.node_config.mparty_port().clone() as u32);
            req.host_address = Some(relay.node_config.external_ip.clone()); // Some("127.0.0.1".to_string());
            req.host_key = Some(relay.node_config.public_key().clone());
            req.store_proof = Some(true);

            let mut hm: HashMap<Vec<u8>, usize> = HashMap::default();
            for (i, pk) in keygen.identifier.safe_get()?.party_keys.iter().enumerate() {
                hm.insert(pk.bytes()?, i + 1);
            };
            let mut parties = vec![];
            for pk in req.identifier.clone().unwrap().party_keys {
                if let Some(i) = hm.get(&pk.bytes()?) {
                    parties.push(i.clone() as i64);
                }
            }
            req.party_indexes = parties;

            let result = initiate_mp_keysign(relay.clone(), req).await?;
            response.initiate_multiparty_signing_response = Some(result);
        }
        // if add_peer_full_request.is_some() {
        //     let add: AddPeerFullRequest = add_peer_full_request.unwrap();
        //     let res = relay.ds.insert_peer_single(
        //         &add.id,
        //         add.trust,
        //         &add.public_key,
        //         add.address.clone(),
        //     );
        //     success = res.is_ok();
        //     if res.is_ok() {
        //         if add.connect_to_peer {
        //             info!("Dialing address: {}", add.address.clone());
        //             // block_on(p2p_client2.dial(
        //             //     to_libp2p_peer_id(
        //             //         &PublicKey::from_slice(&*add.public_key).unwrap(),
        //             //     ),
        //             //     add.address.parse().unwrap(),
        //             // ))
        //             // .expect("done");
        //         }
        //     } else {
        //         error!("Error {}", res.err().unwrap());
        //     }
        // }
        Ok(response)
    }

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
        let rt2 = runtime.clone();
        let control_relay = relay.clone();
        let control_single_json = warp::post()
            .and(warp::path("control"))
            // Only accept bodies smaller than 16kb...
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json::<ControlRequest>())
            .and_then(move |req: ControlRequest| {
                let rt3 = rt2.clone();
                let rl2 = control_relay.clone();
                async move {
                    let relay_int = rl2.clone();
                    Self::handle_control(req, relay_int, rt3).await
                }
            });

        warp::serve(control_single_json)
            .run(([127, 0, 0, 1], relay2.node_config.control_port()))
            .await;
        Ok(())
    }

    async fn handle_control(req: ControlRequest, relay_int: Relay, rt: Arc<Runtime>) -> Result<Json, Rejection> {
        let mut response =
            Self::request_response(req, relay_int.clone(), rt.clone()).await;
        let res = response.map_err(|e| {
            let mut r = ControlResponse::empty();
            r.response_metadata = Some(e.response_metadata());
            r
        });
        as_warp_json_response(res)
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
