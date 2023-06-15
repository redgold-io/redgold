use itertools::Itertools;
use redgold_schema::{ProtoSerde, RgResult, SafeBytesAccess, SafeOption, WithMetadataHashable};
use crate::api::hash_query::hash_query;
use crate::core::relay::Relay;
use serde::{Serialize, Deserialize};
use redgold_schema::structs::{ErrorInfo, QueryTransactionResponse, State, SubmitTransactionResponse, Transaction};

#[derive(Serialize, Deserialize)]
pub struct HashResponse {
    pub hash: String,
    pub height: u64,
    pub timestamp: u64,
    pub transactions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BriefTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub fee: f64,
    pub bytes: i64,
    pub timestamp: i64,
}



#[derive(Serialize, Deserialize)]
pub struct PeerSigner{
    pub peer_id: String,
    pub signature: String,
    pub node_id: String,
    pub trust: f64,
}


#[derive(Serialize, Deserialize)]
pub struct DetailedInput {
    pub transaction_hash: String,
    pub output_index: i64,
    pub address: String
}

#[derive(Serialize, Deserialize)]
pub struct DetailedOutput {
    pub output_index: i32,
    pub address: String,
    pub available: bool,
    pub amount: f64,
}


#[derive(Serialize, Deserialize)]
pub struct DetailedTransaction {
    pub info: BriefTransaction,
    /// Normalized to mimic conventional confirmations, i.e. number of stacked observations
    /// Average weighted trust by peer observations
    pub confirmation_score: f64,
    pub acceptance_score: f64,
    pub message: String,
    pub num_pending_signers: i64,
    pub num_accepted_signers: i64,
    pub accepted: bool,
    pub signers: Vec<PeerSigner>,
    pub inputs: Vec<DetailedInput>,
    pub outputs: Vec<DetailedOutput>,
    pub rejection_reason: Option<ErrorInfo>,
    pub signable_hash: String,
}


#[derive(Serialize, Deserialize)]
pub struct DetailedAddress {
    pub address: String
}


#[derive(Serialize, Deserialize)]
pub struct ExplorerHashSearchResponse {
    pub transaction: Option<DetailedTransaction>,
    pub address: Option<DetailedAddress>
}

#[derive(Serialize, Deserialize)]
pub struct RecentDashboardResponse {
    pub recent_transactions: Vec<BriefTransaction>
}

pub async fn handle_explorer_hash(hash_input: String, p1: Relay) -> RgResult<ExplorerHashSearchResponse>{
    let hq = hash_query(p1, hash_input).await?;
    let mut h = ExplorerHashSearchResponse{
        transaction: None,
        address: None,
    };
    if let Some(t) = hq.transaction_info {
        let tx = t.transaction.safe_get_msg("Missing transaction but have transactionInfo")?;
        // For confirmation score, should we store that internally in the database or re-calculate it?
        let message = tx.options
            .clone()
            .and_then(|o| o.data.and_then(|d| d.message))
            .unwrap_or("".to_string());

        let mut signers = vec![];
        for s in &t.observation_proofs {
            if let Some(p) = &s.proof {
                if let (Some(pk), Some(sig)) = (&p.public_key, &p.signature) {
                    let peer_signer = PeerSigner{
                        // TODO: query peer ID from peer store
                        peer_id: "".to_string(),
                        signature: hex::encode(sig.bytes.safe_bytes()?),
                        node_id: pk.hex_or(),
                        trust: 1.0,
                    };
                    signers.push(peer_signer);
                }
            }
        }

        let mut inputs = vec![];
        for i in &tx.inputs {
            let input = DetailedInput{
                transaction_hash: i.transaction_hash.clone().map(|t| t.hex()).safe_get_msg("Missing transaction hash?")?.clone(),
                output_index: i.output_index.clone(),
                address: i.address()?.render_string()?,
            };
            inputs.push(input);
        }
        let mut outputs = vec![];
        for (i, o) in tx.outputs.iter().enumerate() {
            let output = DetailedOutput{
                output_index: i.clone() as i32,
                address: o.address.safe_get()?.render_string()?,
                available: t.valid_utxo_index.contains(&(i as i32)),
                amount: o.rounded_amount(),
            };
            outputs.push(output);
        }


        // TODO: Make this over Vec<ObservationProof> Instead
        let mut submit_response = SubmitTransactionResponse::default();
        let mut query_transaction_response = QueryTransactionResponse::default();
        query_transaction_response.observation_proofs = t.observation_proofs.clone().iter().map(|o| o.clone()).collect_vec();
        submit_response.query_transaction_response = Some(query_transaction_response);
        submit_response.transaction = Some(tx.clone());
        let counts = submit_response.count_unique_by_state()?;

        let num_pending_signers = counts.get(&(State::Pending as i32)).unwrap_or(&0).clone() as i64;
        let num_accepted_signers = counts.get(&(State::Finalized as i32)).unwrap_or(&0).clone() as i64;
        let mut detailed = DetailedTransaction{
            info: brief_transaction(tx)?,
            confirmation_score: 1.0,
            acceptance_score: 1.0,
            message,
            num_pending_signers,
            num_accepted_signers,
            accepted: t.accepted,
            signers,
            inputs,
            outputs,
            rejection_reason: t.rejection_reason,
            signable_hash: tx.signable_hash().hex(),
        };
        h.transaction = Some(detailed)
    }
    Ok(h)
}

fn brief_transaction(tx: &Transaction) -> RgResult<BriefTransaction> {
    Ok(BriefTransaction {
        hash: tx.hash_or().hex(),
        from: tx.first_input_address()
            .and_then(|a| a.render_string().ok())
            .unwrap_or("".to_string()),
        to: tx.first_output_address().safe_get_msg("Missing output address")?.render_string()?,
        amount: tx.total_output_amount_float(),
        fee: 0f64, // Replace with find fee address?
        bytes: tx.proto_serialize().len() as i64,
        timestamp: tx.struct_metadata.clone().and_then(|s| s.time).safe_get_msg("Missing tx timestamp")?.clone(),
    })
}


pub async fn handle_explorer_recent(r: Relay) -> RgResult<RecentDashboardResponse>{
    let recent = r.ds.transaction_store.query_recent_transactions(Some(10)).await?;
    let mut recent_transactions = Vec::new();
    for tx in recent {
        let brief_tx = brief_transaction(&tx)?;
        recent_transactions.push(brief_tx);
    }
    Ok(RecentDashboardResponse {
        recent_transactions,
    })
}

