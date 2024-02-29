use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use redgold_schema::structs::{ErrorInfo, Hash, Input, ObservationProof, Output, PartitionInfo, PublicKey, Request, ResolveHashRequest, Response, Transaction};
use crate::core::relay::Relay;

use async_trait::async_trait;
use futures::{future, TryFutureExt};
use itertools::Itertools;
use log::info;
use tokio::runtime::Runtime;
use redgold_keys::transaction_support::InputSupport;
use redgold_schema::{error_info, ErrorInfoContext, ProtoHashable, RgResult, SafeOption, WithMetadataHashable};
use crate::core::resolve::resolve_output::ResolvedOutputChild;
use crate::genesis::create_test_genesis_transaction;
use redgold_schema::EasyJson;
use redgold_schema::errors::EnhanceErrorInfo;
use crate::core::internal_message::SendErrorInfo;

#[async_trait]
trait SingleResolver {
    async fn resolve(
        &self,
        relay: Relay,
        runtime: Arc<Runtime>,
        peers: Vec<PublicKey>
    ) -> Result<ResolvedInput, ErrorInfo>;
}

// impl PartialEq for Transaction {
//
// }

#[derive(Clone)]
pub struct ResolvedInput {
    pub input: Input,
    pub parent_transaction: Transaction,
    pub internal_accepted: bool,
    pub internal_valid_index: bool,
    pub observation_proofs: HashSet<ObservationProof>,
    pub peer_valid_index: HashSet<PublicKey>,
    pub peer_invalid_index: HashSet<PublicKey>,
    pub signable_hash: Hash,
}


impl ResolvedInput {

    pub fn prior_output(&self) -> Result<&Output, ErrorInfo> {
        self.parent_transaction.outputs.get(self.input.utxo_id.safe_get_msg("missing utxoid")?.output_index as usize)
            .ok_or(ErrorInfo::error_info("Output index out of bounds"))
    }

    /// Ignore validations already covered by the construction of this instance
    /// I.e. internally accepted but with no valid utxos
    pub fn verify_proof(&self) -> Result<(), ErrorInfo> {

        let prior_output = self.prior_output()?;
        // TODO: Actually verify parent transaction hash matches input here right ?
        // Or is this done already?
        self.input.verify_proof(prior_output.address.safe_get()?, &self.signable_hash)?;

        Ok(())
    }
    pub fn amount(&self) -> Result<Option<i64>, ErrorInfo> {
        let prior_output = self.prior_output()?;
        Ok(prior_output.opt_amount())
    }
}


pub struct ResolvedTransactionHash {
    pub hash: Hash,
    pub transaction: Transaction,
    pub observation_proofs: HashSet<ObservationProof>,
    pub peer_valid_index: HashSet<PublicKey>,
    pub peer_invalid_index: HashSet<PublicKey>,
}

impl ResolvedTransactionHash {
    // TODO: Move validate function here
    pub fn valid_index(&self, _relay: &Relay) {
        // relay.get_trust_of_node()
    }
}

pub async fn resolve_transaction_hash(
    peers: Option<Vec<PublicKey>>,
    relay: &Relay,
    hash: &Hash,
    output_index: Option<i64>,
) -> RgResult<Option<ResolvedTransactionHash>> {
    let all = peers.unwrap_or(relay.ds.peer_store
        .peers_near(hash, |pi| pi.transaction_hash).await?
    );
    let mut request = Request::default();
    let mut resolve_request = ResolveHashRequest::default();
    resolve_request.hash = Some(hash.clone());
    resolve_request.output_index = output_index;
    request.resolve_hash_request = Some(resolve_request);
    // TODO: on failure, notify peer manager to remove peer / consider inactive
    let results = relay.broadcast_async(
        all,
        request,
        Some(Duration::from_secs(10))
    ).await?;
    let mut observation_proofs = HashSet::new();
    let mut peer_valid_index = HashSet::default();
    let mut peer_invalid_index = HashSet::default();
    let mut res: Option<Transaction> = None;

    for result in results {
        if let Some(pk) = result
            .as_ref()
            .ok()
            .and_then(|r| r.proof.as_ref().and_then(|p| p.public_key.clone())) {
            match validate_single_result(hash, result) {
                Ok((tx, proofs, valid_idx)) => {
                    // double check all TX same here.
                    res = Some(tx);
                    observation_proofs.extend(proofs);
                    if valid_idx {
                        peer_valid_index.insert(pk);
                    } else {
                        peer_invalid_index.insert(pk);
                    }
                }
                Err(_) => {
                    metrics::counter!("redgold.transaction.resolve.input.errors").increment(1);
                }
            }
        }
    }
    Ok(res.map(|r| ResolvedTransactionHash {
        hash: hash.clone(),
        transaction: r,
        observation_proofs,
        peer_valid_index,
        peer_invalid_index,
    }))
}


pub fn validate_single_result(
    hash: &Hash,
    response: Result<Response, ErrorInfo>
)
                              -> Result<(Transaction, Vec<ObservationProof>, bool), ErrorInfo> {
    let response = response?;
    response.as_error_info()?;
    let response = response.resolve_hash_response.safe_get_msg("Missing resolve response")?;
    let response = response.transaction_info.safe_get_msg("Missing transaction info")?;
    let tx = response.transaction.safe_get_msg("Missing transaction")?;
    if !tx.calculate_hash().eq(&hash) {
        return Err(ErrorInfo::error_info("Invalid transaction hash"));
    }
    let idx = response.queried_output_index_valid.unwrap_or(false);

    Ok((tx.clone(), response.observation_proofs.clone(), idx))
}

pub async fn resolve_input(
    input: Input, relay: Relay, _peers: Vec<PublicKey>, signable_hash: Hash,
    check_liveness: bool
)
                           -> Result<ResolvedInput, ErrorInfo> {
    metrics::counter!("redgold.transaction.resolve.input").increment(1);
    let u = input.utxo_id.safe_get_msg("Missing utxo id")?;
    let hash = u.transaction_hash.safe_get_msg("Missing transaction hash on input")?;

    // TODO this check can be skipped if we check our XOR distance first.
    // Check if we have the parent transaction stored locally
    let res_terr = relay.ds.transaction_store.query_maybe_transaction(hash).await?;
    let mut res: Option<Transaction> = None;
    match res_terr {
        None => {}
        Some((t, e)) => {
            if let Some(e) = e {
                return Err(e);
            }
            res = Some(t);
        }
    }

    // Check if the UTXO is still valid (even if the transaction is known, it's output may have been used already)
    let internal_valid_index = relay.ds.transaction_store.query_utxo_id_valid(
        hash,
        u.output_index
    ).await?;

    // We have the transaction accepted locally
    let internal_accepted = res.is_some();
    // If we are storing this transaction, then we should also know the UTXOs
    if internal_accepted && !internal_valid_index && check_liveness {
        // We have the transaction stored, but we don't consider it's outputs valid anymore
        return Err(ErrorInfo::error_info("Missing valid UTXO index on accepted transaction"));
    }


    // TODO: Handle the conflicts here by persisting the additional information in response
    // This is also missing dealing with conflicts generated by rejecting a transaction.
    // Keep in mind an important distinction here, if the public key appears in any valid index
    // then it is by definition excluding all the others.
    // Therefore it generates conflicts
    let mut observation_proofs = HashSet::new();
    let mut peer_valid_index = HashSet::default();
    let mut peer_invalid_index = HashSet::default();

    let oe = relay.ds.observation.select_observation_edge(hash).await?;
    info!("Num internal observation proofs {} {}", oe.len(), hash.json_or());
    for o in oe {
        observation_proofs.insert(o);
    }

    if !internal_accepted {

        let resolved = resolve_transaction_hash(
            None, &relay, hash, Some(u.output_index)
        ).await?;
        if let Some(r) = resolved {
            observation_proofs = r.observation_proofs.clone();
            peer_valid_index = r.peer_valid_index.clone();
            peer_invalid_index = r.peer_invalid_index.clone();
            res = Some(r.transaction);
        }

        if observation_proofs.len() == 0 {
            return Err(ErrorInfo::error_info("Missing observation proofs"));
        }
        // TODO: Use trust score here
        let invalid_majority = peer_invalid_index.len() > peer_valid_index.len();
        if invalid_majority && check_liveness {
            // TODO: Include error information about which peers rejected it, i.e. a distribution
            return Err(ErrorInfo::error_info("UTXO considered invalid by peer selection"));
        }

        let utxo_invalid = peer_valid_index.is_empty() && !internal_valid_index;
        if utxo_invalid && check_liveness {
            return Err(ErrorInfo::error_info("No peers considered UTXO valid"));
        }

    }

    let resolved = ResolvedInput {
        input: input.clone(),
        parent_transaction: res.ok_or(ErrorInfo::error_info("Missing parent transaction"))?,
        internal_accepted,
        internal_valid_index,
        observation_proofs,
        peer_valid_index,
        peer_invalid_index,
        signable_hash
    };
    resolved.verify_proof()?;
    Ok(resolved)
}
// }



#[async_trait]
pub trait RuntimeRunErrorInfo {
    async fn spawn_err<F>(&self, future: F) -> Result<F::Output, ErrorInfo>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static;
}

#[async_trait]
impl RuntimeRunErrorInfo for Arc<Runtime> {
    async fn spawn_err<F>(&self, future: F) -> Result<F::Output, ErrorInfo>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
    {
        self.spawn(future).await.error_info("Join handle failure on future")
    }
}



pub struct ResolvedTransaction {
    /// Transaction under consideration, represents the active thread request and also the
    /// child transaction of the resolved parents.
    transaction: Transaction,
    /// Already undergone localized validation (i.e. proofs, peer queries, etc.)
    fixed_resolutions: Vec<ResolvedInput>,
    pub resolved_internally: bool,
    pub descendents: Vec<ResolvedOutputChild>
}

impl ResolvedTransaction {

    pub fn total_parent_amount_available(&self) -> Result<i64, ErrorInfo> {
        let mut total = 0;
        for r in &self.fixed_resolutions {
            let result = r.amount()?;
            if let Some(a) = result {
                total += a;
            }
        }
        Ok(total)
    }

    pub fn validate(&self) -> Result<(), ErrorInfo> {
        let requested_total = self.transaction.total_output_amount();
        let available_total = self.total_parent_amount_available()?;
        if available_total != requested_total {
            return Err(ErrorInfo::error_info("Balance mismatch"));
        }
        Ok(())
    }
}

//
// #[async_trait]
// pub trait Resolver {
//     async fn resolve(&'static self, relay: Relay, runtime: Arc<Runtime>) -> Result<Vec<Transaction>, ErrorInfo>;
// }
//
// #[async_trait]
// impl Resolver for Transaction {
// TODO: This should also trigger downloads etc. / acceptance
pub async fn resolve_transaction(tx: &Transaction, relay: Relay
                                 // , runtime: Arc<Runtime>
) -> Result<ResolvedTransaction, ErrorInfo> {
    let peers = relay.ds.peer_store.active_nodes(None).await?;
    let mut resolved_internally = true;
    let mut vec = vec![];

    // TODO: Have we verified this input contains all the signatures?
    for result in future::join_all(tx.inputs.iter()
        .filter(|i| i.floating_utxo_id.is_none())
        .map(|input|
        async{tokio::spawn(resolve_input(input.clone(), relay.clone(),
                                        // runtime.clone(),
                                         peers.clone(), tx.signable_hash().clone(), true))
            .await.map_err(|e| error_info(e.to_string()))}
    ).collect_vec()).await {
        let result = result??;
        if !result.internal_accepted {
            relay.unknown_resolved_inputs.sender.send_err(result.clone()).mark_abort()?;
            resolved_internally = false;
        }
        vec.push(result)
    }
    let resolved = ResolvedTransaction {
        transaction: tx.clone(),
        fixed_resolutions: vec,
        resolved_internally,
        descendents: vec![],
    };
    Ok(resolved)
    }
// }

#[test]
fn test_hashmap() {
    let tx = create_test_genesis_transaction();
    let h = tx.calculate_hash();
    let mut m: HashMap<Hash, Transaction> = HashMap::default();
    m.insert(h.clone(), tx.clone());
    let result = m.get(&h);
    let t2 = result.expect("transaction not found");
    assert_eq!(t2, &tx);
}