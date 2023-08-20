use std::collections::HashSet;
use std::time::Duration;
use itertools::Itertools;
use redgold_schema::{error_info, RgResult, SafeBytesAccess, SafeOption};
use redgold_schema::structs::{Address, ErrorInfo, Hash, Input, NodeMetadata, ObservationProof, Output, PublicKey, Request, ResolveCodeResponse, Response, Transaction};
use redgold_schema::util::xor_distance::XorConvDistanceSubset;
use crate::core::relay::Relay;

pub struct ResolvedOutputChild {
    output: Output,
    resolve: ResolveCodeResponse
    // internal_accepted: bool,
    // internal_valid_index: bool,
    // observation_proofs: HashSet<ObservationProof>,
    // peer_valid_index: HashSet<PublicKey>,
    // peer_invalid_index: HashSet<PublicKey>,
}
pub async fn resolve_output(
    output: Output, relay: Relay, peers: Vec<NodeMetadata>
)  -> RgResult<ResolvedOutputChild> {
    metrics::increment_counter!("redgold.transaction.resolve.output");

    let pay_desc = output.pay_update_descendents();
    if !pay_desc {
        return Err(error_info("Attempting to resolve transaction output children which does not require update"))
    }

    let addr = output.address.safe_get_msg("Missing address")?;

    // TODO this check can be skipped if we check our XOR distance first.
    // Check if we have the parent transaction stored locally
    // relay.ds.transaction_store.
    let mut internal_resolve = relay.ds.resolve_code(addr).await?;

    // Add xor distance here

    // TODO: Handle the conflicts here by persisting the additional information in response
    // This is also missing dealing with conflicts generated by rejecting a transaction.
    // Keep in mind an important distinction here, if the public key appears in any valid index
    // then it is by definition excluding all the others.
    // Therefore it generates conflicts
    // let mut observation_proofs = HashSet::new();
    // let mut peer_valid_index = HashSet::default();
    // let mut peer_invalid_index = HashSet::default();
    // Do we care about distinguishing the peer contract state markers at this point in time?
    let mut majority_resolve: Option<ResolveCodeResponse> = None;

    if internal_resolve.transaction.is_none() {

        let peer_subset: Vec<&NodeMetadata> = peers.xor_conv_distance_subset(
            &addr.address.safe_bytes()?, |i| i.contract_address
        );
        let peer_keys = peer_subset.iter().filter_map(|n| n.public_key.clone()).collect_vec();

        let mut request = Request::default();
        let mut resolve_request = addr.clone();
        request.resolve_code_request = Some(resolve_request);

        let results = relay.broadcast_async(peer_keys.clone(), request, Some(Duration::from_secs(10))).await?;

        for (result, pk) in results.iter().zip(peer_keys.iter()) {
            match validate_resolve_output_response(addr, result) {
                Ok(r) => {
                    // double check all TX same here.
                    majority_resolve = Some(r);
                    // observation_proofs.extend(proofs);
                }
                Err(_) => {
                    metrics::increment_counter!("redgold.transaction.resolve.output.errors");
                }
            }
        }
        // if observation_proofs.len() == 0 {
        //     return Err(ErrorInfo::error_info("Missing observation proofs"));
        // }
        // // TODO: Use trust score here
        // if peer_invalid_index.len() > peer_valid_index.len() {
        //     // TODO: Include error information about which peers rejected it, i.e. a distribution
        //     return Err(ErrorInfo::error_info("UTXO considered invalid by peer selection"));
        // }
        //
        // if peer_valid_index.is_empty() {
        //     return Err(ErrorInfo::error_info("No peers considered UTXO valid"));
        // }

    }

    let mut resolve = internal_resolve.clone();
    if resolve.contract_state_marker.is_none() {
        resolve.contract_state_marker = majority_resolve.clone().and_then(|r| r.contract_state_marker);
    }
    if resolve.transaction.is_none() {
        resolve.transaction = majority_resolve.clone().and_then(|r| r.transaction);
    }
    if resolve.utxo_entry.is_none() {
        resolve.utxo_entry = majority_resolve.clone().and_then(|r| r.utxo_entry);
    }

    let resolved = ResolvedOutputChild {
        output,
        resolve,
    };
    Ok(resolved)
}

pub fn validate_resolve_output_response(code_address: &Address, response: &RgResult<Response>) -> RgResult<ResolveCodeResponse> {
    let res = response.clone()?;
    let rcr = res.resolve_code_response.safe_get_msg("Missing resolve code response")?;
    if let Some(tx) = rcr.transaction.clone().and_then(|t| t.transaction) {
        let outputs = tx.output_of(code_address);
        let head_output = outputs.get(0);
        let o = head_output.safe_get_msg("Missing head output")?;
        o.validate_deploy_code()?;
        // TODO: Validate the UTXO entry etc.
        Ok(rcr.clone())
    } else {
        Err(ErrorInfo::error_info("Missing transaction in resolve code response"))
    }
}