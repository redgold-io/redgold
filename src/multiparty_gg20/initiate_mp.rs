use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption, structs};
use redgold_schema::structs::{BytesData, ErrorInfo, InitiateMultipartyKeygenRequest, InitiateMultipartyKeygenResponse, InitiateMultipartySigningRequest, InitiateMultipartySigningResponse, LocalKeyShare, MultipartyIdentifier, PartyInfo, PartySigningValidation, Proof, PublicKey, Request, Response, RoomId, Weighting};
use redgold_common::flume_send_help::SendErrorInfo;
use crate::core::relay::Relay;
use futures::{StreamExt, TryFutureExt};
use itertools::Itertools;
use metrics::{counter, gauge};
use serde::{Deserialize, Serialize};
// use ssh2::init;
use tokio::runtime::Runtime;
use uuid::Uuid;
use crate::multiparty_gg20::{gg20_keygen, gg20_signing};

#[test]
fn debug() {

}
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::easy_json::json_pretty;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::conf::node_config::NodeConfig;
use crate::party::event_validator::PartyEventValidator;
use crate::util::current_time_millis_i64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfInitiateKeygenResult {
    pub local_share: String,
    pub identifier: MultipartyIdentifier,
    pub request: InitiateMultipartyKeygenRequest
}

pub fn default_room_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn default_room_id_typed() -> Option<RoomId> {
    Some(RoomId {
        uuid: Some(default_room_id())
    })
}

pub fn default_room_id_signing(keygen_room_id: &RoomId) -> RgResult<RoomId> {
    let signing_id = default_room_id();
    Ok(RoomId {
        uuid: Some(format!("{}_{}", keygen_room_id.uuid.safe_get_msg("Missing uuid")?, signing_id)),
    })
}

pub async fn default_identifier(relay: Relay) -> RgResult<MultipartyIdentifier> {
    let kp = find_multiparty_key_pairs(relay.clone()).await?;
    let num_parties = kp.len();
    let weighting = default_threshold(num_parties);
    let ident = MultipartyIdentifier {
        room_id: Some(RoomId {
            uuid: Some(default_room_id())
        }) ,
        threshold: Some(weighting),
        party_keys: kp,
    };
    Ok(ident)
}

fn default_threshold(num_parties: usize) -> Weighting {
    let thresh_direct = if num_parties <= 5 {
        (num_parties - 1) as i64
    } else {
        (num_parties / 2) as i64
    };
    let weighting = Weighting::from_int_basis(thresh_direct, num_parties as i64);
    weighting
}

pub async fn default_half_identifier(kp: Vec<PublicKey>) -> RgResult<MultipartyIdentifier> {
    let ident = MultipartyIdentifier {
        room_id: default_room_id_typed(),
        threshold: Some(default_threshold(kp.len())),
        party_keys: kp,
    };
    Ok(ident)
}


pub async fn initiate_mp_keygen(
    relay: Relay,
    ident: Option<MultipartyIdentifier>,
    store_local_share: bool,
    node_ids: Option<Vec<PublicKey>>,
    debug_purpose: bool,
    prior_keys: Vec<PublicKey>
) -> Result<SelfInitiateKeygenResult, ErrorInfo> {

    counter!("redgold_initiate_mp_keygen").increment(1);

    // Better pattern for unwrap or else async error?
    let ident = match ident {
        None => {
            match node_ids {
                None => {
                    default_identifier(relay.clone()).await?
                }
                Some(pks) => {
                    let non_self_pks = pks.iter()
                        .filter(|&pk| pk != &relay.node_config.public_key())
                        .cloned()
                        .collect_vec();
                    let mut ids_pks = vec![relay.node_config.public_key()];
                    ids_pks.extend(non_self_pks);
                    default_half_identifier(ids_pks).await?
                }
            }
        }
        Some(x) => {x}
    };

    let mut base_request = InitiateMultipartyKeygenRequest::default();
    base_request.purpose = {
        if debug_purpose {
            structs::PartyPurpose::StandardPurpose
        } else {
            structs::PartyPurpose::StandardPurpose
        }
    } as i32;
    let identifier = ident.clone();
    base_request.identifier = Some(identifier.clone());
    base_request.time = current_time_millis_i64();
    base_request.prior_keys = prior_keys;

    // Need a delete on failure here
    relay.authorize_keygen(base_request.clone())?;

    let result = initiate_mp_keygen_authed(
        relay.clone(), base_request.clone(), store_local_share).await;

    relay.remove_keygen_authorization(&ident.room_id.safe_get_msg("rid")?.clone())?;

    result
}

pub async fn initiate_mp_keygen_authed(
    relay: Relay,
    base_request: InitiateMultipartyKeygenRequest,
    store_local_share: bool
) -> Result<SelfInitiateKeygenResult, ErrorInfo> {

    let ident = base_request.identifier.safe_get_msg("No identifier")?.clone();
    let index = 1u16;
    let number_of_parties = ident.party_keys.len() as u16;
    let threshold = ident.threshold.safe_get()?.value.clone() as u16;
    let room_id = ident.room_id.safe_get_msg("rid")?.uuid.safe_get()?.clone();
    let address = "127.0.0.1".to_string();
    let port = relay.node_config.mparty_port();
    // TODO: From nodeconfig?
    let timeout = relay.node_config.multiparty_timeout();

    // TODO: First query all peers to determine if they are online.
    let self_key = relay.node_config.public_key();


    // Does this fix the issue? Introducing a sleep BEFORE this reliably produces the error
    // Try introducing it AFTER
    info!("Initiating mp keygen starter for room: {} with index: {} num_parties: {}, threshold: {}, port: {}",
        room_id, index.to_string(), number_of_parties.to_string(), threshold.to_string(), port.to_string());
    let ridc = room_id.clone();
    let nc = relay.clone();
    let res = tokio::spawn(async move {
        tokio::time::timeout(
            timeout,
            gg20_keygen::keygen(address, port, ridc, index, threshold, number_of_parties, nc),
        ).await.map_err(|_| error_info("Timeout"))
    });
    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut req = Request::empty();
    req.initiate_keygen = Some(base_request.clone());
    let peers = ident.party_keys.iter().filter(|&p| p != &self_key)
        .map(|x| x.clone())
        .collect_vec();
    // info!("Sending initiate keygen request to peers: {} message: {}", peers.json_or(), req.json_or());

    let results = relay.broadcast_async(peers.clone(), req, Some(relay.node_config.multiparty_timeout())).await?;

    let mut successful = 0;

    for result in results {

        // info!("Received initiate keygen response from peer: {:?}", result);
        let res = result.clone()
            .and_then(|r| r.as_error_info());
        match res {
            Ok(_r) => {
                successful += 1
            }
            Err(e) => {
                use redgold_schema::helpers::easy_json::json_or;
                // TODO: add peer short public key identifier.
                info!("Error sending initiate keygen request to peer {}", json_or(&e));
            }
        }
    }

    // info!("Num successful peers participating in keygen: {}", successful);

    if successful < peers.len() {
        res.abort();
        return Err(error_info("Not enough successful peers"));
    }
    let local_share = res.await.error_info("join handle error")???;

    if store_local_share {

        let pi = PartyInfo::new_from(base_request.clone(), local_share.clone(), true);
        // info!("Storing local share for room: {}", room_id.clone());
        relay.ds.multiparty_store.add_keygen(
            &pi,
        ).await?;
        let query_check = relay.ds.multiparty_store.party_info(&RoomId::from(room_id.clone())).await?;
        query_check.safe_get_msg("Unable to query local store for room_id on keygen")?;
        // info!("Local share confirmed");
    }
    let response1 = SelfInitiateKeygenResult{
        local_share,
        identifier: ident,
        request: base_request,
    };
    Ok(response1)
}



pub async fn initiate_mp_keygen_follower(
    relay: Relay, mp_req: InitiateMultipartyKeygenRequest, initiating_pk: &PublicKey
)
                                -> Result<InitiateMultipartyKeygenResponse, ErrorInfo> {

    // TODO: Verify score of initiating pk

    let score = relay.get_security_rating_trust_of_node(initiating_pk).await?.ok_msg("Initiating public key trust not found")?;
    // TODO: use a function on a score class rather than this explicit check
    if score < 0.1 {
        return Err(error_info("Initiating public key trust score too low"));
    }

    let ident = mp_req.identifier.safe_get()?;
    let index = ident.party_keys.iter().enumerate().find(|(_idx, x)| x == &&relay.node_config.public_key())
        .map(|(idx, _x)| (idx as u16) + 1)
        .ok_or(error_info("Not a participant"))?;
    // TODO: Verify address matches host key
    // let key = mp_req.host_key.safe_get()?.clone();
    let number_of_parties = ident.party_keys.len() as u16;
    let threshold = ident.threshold.safe_get()?.value as u16;
    let room_id = ident.room_id.safe_get_msg("rid")?.uuid.safe_get()?.clone();

    let host_key = ident.party_keys.get(0).cloned();
    let host_key = host_key.safe_get_msg("No host key")?;
    if initiating_pk != host_key {
        return Err(error_info("Initiating public key does not match host key"));
    }
    let metadata = relay.ds.peer_store.query_public_key_metadata(host_key).await?;
    let metadata = metadata.safe_get_msg("No host key metadata")?;
    let address = metadata.external_address()?;
    let port = metadata.port_or(relay.node_config.network) + 4u16;

    let timeout = relay.node_config.multiparty_timeout();

    // info!("Initiating mp keygen follower for room: {} with index: {} num_parties: {}, threshold: {}, port: {}",
    //     room_id, index.to_string(), number_of_parties.to_string(), threshold.to_string(), port.to_string());
    let config = relay.clone();
    let local_share_result = tokio::time::timeout(
        timeout,
        gg20_keygen::keygen(address, port, room_id.clone(), index, threshold, number_of_parties, config),
    ).await.map_err(|_| error_info("Timeout"))??;

    // info!("Storing local share on follower for room: {}", room_id.clone());

    let pi = PartyInfo::new_from(mp_req.clone(), local_share_result.clone(), false);
    relay.ds.multiparty_store.add_keygen(
        &pi
    ).await?;
    let query_check = relay.ds.multiparty_store.party_info(&RoomId::from(room_id.clone())).await?;
    query_check.safe_get_msg("Unable to query local store for room_id on keygen")?;
    // info!("Local share confirmed on follower ");
    // relay.ds.multiparty_store.add_keygen(res, room_id.clone(), mp_req.clone()).await?;
    Ok(InitiateMultipartyKeygenResponse{ initial_request: Some(mp_req.clone()) })
}


// TODO: Change this to a health request rather than about
// Also this is generic enough to move over to the relay directly
// also change it to a broadcast.
pub async fn find_multiparty_key_pairs(relay: Relay
                                       // , runtime: Arc<Runtime>
) -> Result<Vec<structs::PublicKey>, ErrorInfo> {

    let peers = relay.ds.peer_store.all_peers().await?;
    // TODO: Safer, query all pk
    let pk =
        peers.iter().map(|p| p.node_metadata.get(0).clone().unwrap().public_key.clone().unwrap())
            .filter(|p| {
                if p == &relay.node_config.public_key() {
                    error!("Found self in peer list");
                    false
                } else {
                    true
                }
            })
            .collect_vec();

    // info!("Multiparty found {} possible peers", pk.len());
    let results = Relay::broadcast(relay.clone(),
        pk, Request::empty().about(),
                                   // runtime.clone(),
                                   Some(Duration::from_secs(60))
    ).await;
    // Check if response also is not error info
    let valid_pks = results.iter()
        .filter_map(|(pk, r)| if r.as_ref().ok().filter(|r| r.as_error_info().is_ok())
            .is_some() { Some(pk.clone()) } else { None })
        .collect_vec();
    // TODO: Separate this type of error here instead to be optional only converted later
    // info!("Multiparty found {} valid_pks peers", valid_pks.len());
    if valid_pks.len() == 0 {
        return Err(ErrorInfo::error_info("No valid peers found"));
    }
    let mut keys = vec![relay.node_config.public_key()];
    keys.extend(valid_pks.clone());
    Ok(keys)
}


pub fn fill_identifier(keys: Vec<structs::PublicKey>, identifier: Option<MultipartyIdentifier>) -> Option<MultipartyIdentifier> {
    let num_parties = keys.len() as i64;
    if let Some(ident) = identifier {
        let mut identifier = ident;
        identifier.room_id = default_room_id_typed();
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
        let threshold = Some(Weighting::from_int_basis(threshold, num_parties));
        Some(
            MultipartyIdentifier {
                party_keys: keys.clone(),
                threshold,
                room_id: default_room_id_typed(),
            }
        )
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfInitiateKeysignResult {
    pub ident: MultipartyIdentifier,
    pub signing_room_id: RoomId,
    pub parties: Vec<PublicKey>,
    pub proof: Proof
}


pub async fn initiate_mp_keysign(
    relay: Relay,
    ident: MultipartyIdentifier,
    // Change to &Vec<u8>?
    data_to_sign: BytesData,
    mut parties: Vec<PublicKey>,
    signing_room_id: Option<RoomId>,
    validation: Option<PartySigningValidation>
) -> RgResult<SelfInitiateKeysignResult> {


    if parties.is_empty() {
        parties = find_multiparty_key_pairs(relay.clone()).await?;
    }

    // Ensure that default starts with keygen UUID to avoid signing wrong hash
    // TODO: I don't think this is even necessary on the room id is it? maybe not or maybe for auth on request?
    let signing_room_id = signing_room_id
        .unwrap_or(default_room_id_signing(
            ident.room_id.safe_get_msg("missing ident room id")?
        )?);

    let mut mp_req = InitiateMultipartySigningRequest::default();
    mp_req.identifier = Some(ident.clone());
    mp_req.data_to_sign = Some(data_to_sign.clone());
    mp_req.signing_room_id = Some(signing_room_id.clone());
    mp_req.signing_party_keys = parties.clone();
    mp_req.party_signing_validation = validation;

    relay.authorize_signing(mp_req.clone())?;

    let res = initiate_mp_keysign_authed(relay.clone(), mp_req.clone()).await;
    relay.remove_signing_authorization(&signing_room_id.clone())?;
    // Err(error_info("debug"))
    res
}

pub async fn initiate_mp_keysign_authed(
    relay: Relay,
    mp_req: InitiateMultipartySigningRequest
) -> RgResult<SelfInitiateKeysignResult> {

    let ident = mp_req.identifier.safe_get_msg("Missing identifier")?.clone();
    let _data_to_sign = mp_req.data_to_sign.safe_get_msg("Missing data")?.clone();
    let parties = mp_req.signing_party_keys.clone();

    let signing_room_id = mp_req.signing_room_id.clone();

    let address = "127.0.0.1".to_string();
    let port = relay.node_config.mparty_port();
    let timeout = Duration::from_secs(200 as u64);
    let init_keygen_req_room_id_typed = ident.room_id.safe_get()?;
    let init_keygen_req_room_id = ident.room_id.safe_get()?.uuid.safe_get()?.clone();
    let index = ident.party_keys.iter().enumerate().filter_map(|(idx, pk)| {
        if parties.contains(pk) {
            let idx = idx + 1;
            Some(idx as u16)
        } else {
            None
        }
    }).collect_vec();
//let (local_share, init)

    let pi = relay.ds.multiparty_store
        .party_info(init_keygen_req_room_id_typed).await?
        .ok_or(error_info("Local share not found"))?;

    let local_share = pi.local_key_share.safe_get()?.local_share.safe_get_msg("Local share not found")?.clone();

    // TODO: Check initiate keygen matches

    let option = mp_req.data_to_sign.clone().safe_get()?.value.clone();

    let rid = signing_room_id.clone();
    let index2 = index.clone();
    let nc = relay.clone();
    let jh = tokio::spawn(async move { tokio::time::timeout(
        timeout,
        gg20_signing::signing(
            address, port, rid.safe_get()?.uuid.safe_get()?.clone(), local_share.clone(), index2, option, nc),
    ).await.error_info("Timeout")});

    tokio::time::sleep(Duration::from_secs(5)).await;

    let mp_req_external = mp_req.clone();
    let mut req = Request::empty();
    req.initiate_signing = Some(mp_req_external);

    let self_key = relay.node_config.public_key();
    let peers = parties.iter().filter(|&p| p != &self_key)
        .map(|x| x.clone())
        .collect_vec();

    info!("Sending initiate keysign request to peers: {} message: {} self_port: {} signing_room_id: {} index: {}",
        peers.json_or(), req.json_or(), port, signing_room_id.clone().and_then(|x| x.uuid).unwrap_or("".to_string()), index.json_or());

    let results = relay.broadcast_async(peers.clone(), req, Some(relay.node_config.multiparty_timeout())).await?;

    let mut successful = 0;

    for result in results {
        let res = result
            .and_then(|r| r.as_error_info());
        match res {
            Ok(_) => {
                successful += 1
            }
            Err(e) => {
                // info!("Error sending initiate keygen request to peer {}", e.json_or());
            }
        }
    }
    if successful < peers.clone().len() {
        jh.abort();
        return Err(error_info("Not enough successful peers"));
    }

    let proof = jh.await.error_info("join handle error")???;

    relay.ds.multiparty_store.add_signing_proof(
        init_keygen_req_room_id_typed, signing_room_id.safe_get()?, proof.clone(), mp_req.clone()
    ).await?;
    let response1 = SelfInitiateKeysignResult{
        ident,
        signing_room_id: signing_room_id.safe_get()?.clone(),
        parties,
        proof,
    };
    Ok(response1)
}

pub async fn initiate_mp_keysign_follower(
    relay: Relay, mp_req: InitiateMultipartySigningRequest,
    initiating_pk: &PublicKey
)
    -> Result<InitiateMultipartySigningResponse, ErrorInfo> {

    let t = relay.get_security_rating_trust_of_node(initiating_pk).await?.ok_msg("Initiating public key trust not found")?;
    if t < 0.1 {
        return Err(error_info("Initiating keysign public key trust score too low"));
    }


    let ident = mp_req.identifier.safe_get_msg("Missing room id for keygen on signing follower")?;

    let kg_rid_typed = ident.room_id.safe_get()?;
    let keygen_room_id = kg_rid_typed.uuid.safe_get()?.clone();

    // TODO: Duplicated, put on the identifier class
    let index = ident.party_keys.iter().enumerate().filter_map(|(idx, pk)| {
        if mp_req.signing_party_keys.contains(pk) {
            let idx = idx + 1;
            Some(idx as u16)
        } else {
            None
        }
    }).collect_vec();
    // TODO: Verify host key matches address -- do this in request/response API? or maybe here as a param
    // let key = mp_req.host_key.safe_get()?.clone();
    // let number_of_parties = ident.num_parties as u16;
    let signing_room_id = mp_req.signing_room_id.clone();
    let host_key = mp_req.signing_party_keys.get(0).cloned();
    let host_key = host_key.safe_get_msg("No host key")?;
    let metadata = relay.ds.peer_store.query_public_key_metadata(host_key).await?;
    let metadata = metadata.safe_get_msg("No host key metadata")?;
    let address = metadata.external_address()?;
    let port = metadata.port_or(relay.node_config.network) + 4u16;

    let timeout = relay.node_config.multiparty_timeout();

    //TODO: This should be returned as immediate failure on the response level instead of going
    // thru process, maybe done as part of health check?
    let party_info = relay.ds.multiparty_store
        .party_info(&kg_rid_typed).await?
        .ok_or(error_info("Local share not found"))?;

    let signing_bytes = mp_req.data_to_sign.clone().safe_get()?.clone().value;

    if let Some(pk) = &party_info.party_key {
        if party_info.not_debug() {
            let validate = mp_req.party_signing_validation.safe_get_msg("No party signing validation")?.clone();
            let value = relay.external_network_shared_data.read().await;
            let data = value.get(pk).ok_or(error_info("No party key data found"))?;
            data.party_events.safe_get_msg("No party events found")?.validate_event(validate, signing_bytes.clone(), &relay).await?;
        }
    }

    // TODO: Check initiate keygen matches
    let local_share = party_info.local_key_share.safe_get()?.local_share.safe_get()?.clone();
    let init = party_info.initiate.safe_get_msg("No initiate keygen")?.clone();

    let ser = init.json_or();
    if !init.identifier.ok_msg("No identifier on stored initiate keygen").add(ser.clone())?.party_keys.contains(&initiating_pk) {
        Err(error_info("Initiating public key not found in stored initiate keygen")).add(ser).add(initiating_pk.json_or())?
    }

    info!("Initiating follower keysign for \
    room {} with parties {} address: {} port: {} host_key: {}",
        signing_room_id.safe_get()?.uuid.safe_get()?.clone(), index.clone().json_or(), address, port, host_key.json_or()
    );
    let nc = relay.clone();
    let res = tokio::time::timeout(
        timeout,
        gg20_signing::signing(
            address, port, signing_room_id.safe_get()?.uuid.safe_get()?.clone(), local_share, index, signing_bytes, nc),
    ).await.error_info("Timeout")??;

    relay.ds.multiparty_store.add_signing_proof(
        &kg_rid_typed, signing_room_id.safe_get()?, res.clone(), mp_req.clone(),
    ).await?;

    let response = InitiateMultipartySigningResponse { proof: Some(res), initial_request: Some(mp_req.clone()) };
    Ok(response)
}

#[test]
fn run_all() {



}