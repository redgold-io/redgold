// use log::info;
// use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
// use round_based::AsyncProtocol;
// use redgold_schema::{error_info, json_pretty, SafeOption};
// use redgold_schema::structs::{ErrorInfo, InitiateMultipartyKeygenRequest, InitiateMultipartyKeygenResponse, InitiateMultipartySigningRequest, InitiateMultipartySigningResponse};
// use crate::core::internal_message::SendErrorInfo;
// use crate::core::relay::{MultipartyRequestResponse, Relay};
// use crate::mparty::multiparty_client::{join_computation, SmClient};
// use futures::StreamExt;
//
// pub async fn initiate_mp_keygen(relay: Relay, mp_req: InitiateMultipartyKeygenRequest)
//                                 -> Result<InitiateMultipartyKeygenResponse, ErrorInfo> {
//
//     let ident = mp_req.identifier.safe_get()?;
//     let key = mp_req.host_key.unwrap_or(relay.node_config.public_key());
//     let index = mp_req.index.unwrap_or(0) as u16;
//     let number_of_parties = ident.num_parties as u16;
//     let threshold = ident.threshold as u16;
//     let room_id = ident.uuid.clone();
//
//     info!("starting join computation");
//     let sm_client = SmClient::new(relay.clone(), key, room_id);
//
//     let (_i, incoming, outgoing) =
//         join_computation(&sm_client, &sm_client.sub_channel.receiver)
//         .await?;
//
//     info!("Finished join computation");
//
//     let incoming = incoming.fuse();
//     tokio::pin!(incoming);
//     tokio::pin!(outgoing);
//
//     let keygen = Keygen::new(index, threshold, number_of_parties).map_err(|e|
//         error_info(format!("keygen creation failed: {}", e.to_string()))
//     )?;
//     let output = AsyncProtocol::new(keygen, incoming, outgoing)
//         .run()
//         .await
//         .map_err(|e|
//             error_info(format!("protocol execution terminated with error: {:?}", e))
//         )?;
//     let output = json_pretty(&output)?;
//
//
//     //relay.initiate_mp_keysign(mp_req).await
//     Ok(InitiateMultipartyKeygenResponse{ local_share: None })
// }
//
// pub async fn initiate_mp_keysign(relay: Relay, mp_req: InitiateMultipartySigningRequest)
//     -> Result<InitiateMultipartySigningResponse, ErrorInfo> {
//     //relay.initiate_mp_keysign(mp_req).await
//     Ok(InitiateMultipartySigningResponse{ signature: None })
// }
