use std::sync::Arc;
use std::time::Duration;
use async_std::prelude::FutureExt;
use log::info;

use redgold_schema::{error_info, ErrorInfoContext, json_pretty, SafeBytesAccess, SafeOption};
use redgold_schema::structs::{ErrorInfo, InitiateMultipartyKeygenRequest, InitiateMultipartyKeygenResponse, InitiateMultipartySigningRequest, InitiateMultipartySigningResponse, MultipartyThresholdRequest, Request, Response};
use crate::core::internal_message::SendErrorInfo;
use crate::core::relay::{MultipartyRequestResponse, Relay};
use futures::{StreamExt, TryFutureExt};
use itertools::Itertools;
use ssh2::init;
use tokio::runtime::Runtime;
use crate::multiparty::{gg20_keygen, gg20_signing};

#[test]
fn debug() {

}

pub async fn initiate_mp_keygen(relay: Relay, mp_req: InitiateMultipartyKeygenRequest, rt: Arc<Runtime>)
                                -> Result<InitiateMultipartyKeygenResponse, ErrorInfo> {

    let ident = mp_req.identifier.safe_get()?;
    let key = mp_req.host_key.clone().unwrap_or(relay.node_config.public_key());
    let index = mp_req.index.unwrap_or(1) as u16;
    let number_of_parties = ident.num_parties as u16;
    let threshold = ident.threshold as u16;
    let room_id = ident.uuid.clone();
    let address = mp_req.host_address.clone().unwrap_or("127.0.0.1".to_string());
    let port = mp_req.port.map(|x| x as u16).unwrap_or(relay.node_config.mparty_port());
    let timeout = Duration::from_secs(mp_req.timeout_seconds.unwrap_or(100) as u64);

    let mp_req2 = mp_req.clone();
    // TODO: First query all peers to determine if they are online.
    let self_key = relay.node_config.public_key();


    // Does this fix the issue? Introducing a sleep BEFORE this reliably produces the error
    // Try introducing it AFTER
    info!("Initiating mp keygen starter for room: {} with index: {} num_parties: {}, threshold: {}, port: {}",
        room_id, index.to_string(), number_of_parties.to_string(), threshold.to_string(), port.to_string());
    let ridc = room_id.clone();
    let res = rt.spawn(async move {
        tokio::time::timeout(
            timeout,
            gg20_keygen::keygen(address, port, ridc, index, threshold, number_of_parties),
        ).await.map_err(|_| error_info("Timeout"))
    });
    tokio::time::sleep(Duration::from_secs(5)).await;



    let mut successful = 0;
    // TODO: Ensure party key self is first? or is it not present?
    // TODO: Change this to a broadcast.
    for (i_zero_index, peer) in ident.party_keys.iter().enumerate() {
        // Party index is required to be 1-indexed.
        let i = i_zero_index + 1;
        if peer == &self_key {
            continue;
        }
        let mut req = Request::empty();
        let mut mpt = MultipartyThresholdRequest::empty();
        let mut mp_req_external = mp_req2.clone();
        mp_req_external.index = Some(i as i64);
        // TODO: Distinguish here between separate server and localhost
        mp_req_external.host_address = Some(relay.node_config.external_ip.clone());
        mp_req_external.port = Some(relay.node_config.mparty_port() as u32);

        mpt.initiate_keygen = Some(mp_req_external);
        req.multiparty_threshold_request = Some(mpt);
        info!("Sending initiate keygen request to peer: {}", peer.hex()?);
        let res0 = relay.send_message_sync(req, peer.clone(), None).await;
        info!("Received initiate keygen response from peer: {:?}", res0.clone());
        let res = res0.clone()
            .and_then(|r| r.as_error_info());
        match res {
            Ok(response) => {
                successful += 1
            }
            Err(e) => {
                use crate::schema::json_or;
                // TODO: add peer short public key identifier.
                info!("Error sending initiate keygen request to peer {}", json_or(&e));
            }
        }
    }

    info!("Num successful peers participating in keygen: {}", successful);
    if successful < ident.threshold {
        res.abort();
        return Err(error_info("Not enough successful peers"));
    }
    let res = res.await.error_info("join handle error")???;


    let local_share = if mp_req.return_local_share.unwrap_or(true) {
        Some(res.clone())
    } else {
        None
    };
    if mp_req.store_local_share.unwrap_or(true) {
        info!("Storing local share for room: {}", room_id.clone());
        relay.ds.multiparty_store.add_keygen(res, room_id.clone(), mp_req.clone()).await?;
        let query_check = relay.ds.multiparty_store.local_share_and_initiate(room_id.clone()).await?;
        query_check.safe_get_msg("Unable to query local store for room_id on keygen")?;
        info!("Local share confirmed");
    }
    let mut response1 = InitiateMultipartyKeygenResponse::default();
    response1.local_share = local_share;
    response1.initial_request = Some(mp_req);
    Ok(response1)
}



pub async fn initiate_mp_keygen_follower(relay: Relay, mp_req: InitiateMultipartyKeygenRequest)
                                -> Result<InitiateMultipartyKeygenResponse, ErrorInfo> {

    let ident = mp_req.identifier.safe_get()?;
    // TODO: Verify address matches host key
    let key = mp_req.host_key.safe_get()?.clone();
    let index = mp_req.index.safe_get()?.clone() as u16;
    let number_of_parties = ident.num_parties as u16;
    let threshold = ident.threshold as u16;
    let room_id = ident.uuid.clone();
    let address = mp_req.host_address.safe_get()?.clone();
    let port = mp_req.port.safe_get()?.clone() as u16;
    let timeout = Duration::from_secs(100); // mp_req.timeout_seconds.unwrap_or(100) as u64);

    info!("Initiating mp keygen follower for room: {} with index: {} num_parties: {}, threshold: {}, port: {}",
        room_id, index.to_string(), number_of_parties.to_string(), threshold.to_string(), port.to_string());
    let res = tokio::time::timeout(
        timeout,
        gg20_keygen::keygen(address, port, room_id.clone(), index, threshold, number_of_parties),
    ).await.map_err(|_| error_info("Timeout"))??;

    let local_share = None;
    relay.ds.multiparty_store.add_keygen(res, room_id.clone(), mp_req.clone()).await?;
    Ok(InitiateMultipartyKeygenResponse{ local_share, initial_request: None })
}


pub async fn initiate_mp_keysign(relay: Relay, mp_req: InitiateMultipartySigningRequest)
    -> Result<InitiateMultipartySigningResponse, ErrorInfo> {

    let init_keygen_req = mp_req.keygen_room.safe_get()?.clone();
    let init_keygen_req_room_id = init_keygen_req.identifier.safe_get()?.uuid.clone();
    let ident = mp_req.identifier.safe_get()?;
    let key = mp_req.host_key.clone().unwrap_or(relay.node_config.public_key());
    let index: Vec<u16> = mp_req.party_indexes.iter().map(|p| p.clone() as u16).collect_vec();
    let number_of_parties = ident.num_parties as u16;
    let room_id = ident.uuid.clone();
    let address = mp_req.host_address.clone().unwrap_or("127.0.0.1".to_string());
    let port = mp_req.port.map(|x| x as u16).unwrap_or(relay.node_config.mparty_port());
    let timeout = Duration::from_secs(mp_req.timeout_seconds.unwrap_or(100) as u64);

    let (local_share, initiate_keygen) = relay.ds.multiparty_store
        .local_share_and_initiate(init_keygen_req_room_id.clone()).await?
        .ok_or(error_info("Local share not found"))?;
    // TODO: Check initiate keygen matches

    let self_key = relay.node_config.public_key();
    let mut successful = 0;
    for (i, peer) in ident.party_keys.iter().enumerate() {
        if peer == &self_key {
            continue;
        }
        let mut req = Request::empty();
        let mut mpt = MultipartyThresholdRequest::empty();
        let mut mp_req_external = mp_req.clone();
        mp_req_external.host_address = Some(relay.node_config.external_ip.clone());
        mp_req_external.port = Some(relay.node_config.mparty_port() as u32);

        mpt.initiate_signing = Some(mp_req_external);
        // TODO: Distinguish here between separate server and localhost

        let result = relay.send_message_sync(req, peer.clone(), None).await;
        let res = result
            .and_then(|r| r.as_error_info());
        match res {
            Ok(response) => {
                successful += 1
            }
            Err(e) => {
                use crate::schema::json_or;
                // TODO: add peer short public key identifier.
                info!("Error sending initiate keygen request to peer {}", json_or(&e));
            }
        }
    }
    if successful < ident.threshold {
        return Err(error_info("Not enough successful peers"));
    }

    let option = mp_req.data_to_sign.clone().safe_bytes()?;
    let res = tokio::time::timeout(
        timeout,
        gg20_signing::signing(
            address, port, room_id.clone(), local_share, index, option),
    ).await.map_err(|e| error_info("Timeout"))??;

    if mp_req.store_proof.unwrap_or(true) {
        relay.ds.multiparty_store.add_signing_proof(
            init_keygen_req_room_id, room_id.clone(), res.clone(), mp_req.clone()
        ).await?;
    }
    let mut response1 = InitiateMultipartySigningResponse::default();
    response1.proof = Some(res);
    response1.initial_request = Some(mp_req);
    Ok(response1)
}

pub async fn initiate_mp_keysign_follower(relay: Relay, mp_req: InitiateMultipartySigningRequest)
    -> Result<InitiateMultipartySigningResponse, ErrorInfo> {

    let init_keygen_req = mp_req.keygen_room.safe_get()?.clone();
    let init_keygen_req_room_id = init_keygen_req.identifier.safe_get()?.uuid.clone();
    let ident = mp_req.identifier.safe_get()?;
    // TODO: Verify host key matches address
    let key = mp_req.host_key.safe_get()?.clone();
    let index: Vec<u16> = mp_req.party_indexes.iter().map(|p| p.clone() as u16).collect_vec();
    let number_of_parties = ident.num_parties as u16;
    let room_id = ident.uuid.clone();
    let address = mp_req.host_address.safe_get()?.clone();
    let port = mp_req.port.map(|x| x as u16).safe_get()?.clone();
    let timeout = Duration::from_secs(100);

    //TODO: This should be returned as immediate failure on the response level instead of going
    // thru process, maybe done as part of health check?
    let (local_share, initiate_keygen) = relay.ds.multiparty_store
        .local_share_and_initiate(room_id.clone()).await?
        .ok_or(error_info("Local share not found"))?;
    // TODO: Check initiate keygen matches

    let option = mp_req.data_to_sign.clone().safe_get()?.clone().value;
    let res = tokio::time::timeout(
        timeout,
        gg20_signing::signing(
            address, port, room_id.clone(), local_share, index, option),
    ).await.map_err(|e| error_info("Timeout"))??;

    relay.ds.multiparty_store.add_signing_proof(
        init_keygen_req_room_id, room_id.clone(), res.clone(), mp_req.clone()
    ).await?;

    let response = InitiateMultipartySigningResponse { proof: Some(res), initial_request: None };
    Ok(response)
}

#[test]
fn run_all() {



}