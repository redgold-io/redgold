use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY};
use crate::helpers::with_metadata_hashable::{WithMetadataHashable, WithMetadataHashableFields};
use crate::proto_serde::ProtoHashable;
use crate::structs::TransactionType;
use crate::structs::{Address, CurrencyAmount, ErrorInfo, ExternalTransactionId, FloatingUtxoId, Hash, HashType, Input, NetworkEnvironment, NodeMetadata, Observation, ObservationProof, Output, OutputType, PoWProof, PortfolioRequest, ProductId, Proof, PublicKey, StakeDeposit, StakeRequest, StakeWithdrawal, StandardContractType, StandardData, StandardRequest, StandardResponse, StructMetadata, SupportedCurrency, SwapFulfillment, SwapRequest, Transaction, TransactionOptions, TypedValue, UtxoEntry, UtxoId};
use crate::{bytes_data, error_info, structs, ErrorInfoContext, HashClear, PeerMetadata, RgResult, SafeOption};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

pub const MAX_TRANSACTION_MESSAGE_SIZE: usize = 40;


impl UtxoId {

    pub fn new(hash: &Hash, output_index: i64) -> Self {
        Self {
            transaction_hash: Some(hash.clone()),
            output_index,
        }
    }
    pub fn format_str(&self) -> String {
        format!("UtxoId: {} output: {}", self.transaction_hash.clone().expect("").hex(), self.output_index)
    }

    // This is questionable, should we do this long term for XOR distance?
    pub fn utxo_id_vec(&self) -> Vec<u8> {
        let mut merged: Vec<u8> = vec![];
        merged.extend(self.transaction_hash.clone().expect("hash").vec());
        merged.extend(self.output_index.to_le_bytes().to_vec());
        merged
    }

    pub fn as_hash(&self) -> Hash {
        let mut h = Hash::new_direct_transaction(&self.utxo_id_vec());
        h.hash_type = HashType::UtxoId as i32;
        h
    }
    pub fn utxo_id_hex(&self) -> String {
        hex::encode(self.utxo_id_vec())
    }
}


pub fn amount_to_raw_amount(redgold: u64) -> u64 {
    assert!(redgold <= MAX_COIN_SUPPLY as u64);
    return redgold * (DECIMAL_MULTIPLIER as u64);
}

trait BalanceConversion {
    fn rounded_float(&self) -> f64;
}

impl BalanceConversion for i64 {
    fn rounded_float(&self) -> f64 {
        rounded_balance_i64(*self)
    }
}

pub fn rounded_balance(redgold_amount: u64) -> f64 {
    (redgold_amount as f64) / (DECIMAL_MULTIPLIER as f64)
}

pub fn rounded_balance_i64(redgold_amount: i64) -> f64 {
    (redgold_amount as f64) / (DECIMAL_MULTIPLIER as f64)
}

impl WithMetadataHashableFields for Transaction {
    fn struct_metadata_opt(&mut self) -> Option<&mut StructMetadata> {
        self.struct_metadata.as_mut()
    }

    fn struct_metadata_opt_ref(&self) -> Option<&StructMetadata> {
        self.struct_metadata.as_ref()
    }
}

impl HashClear for Input {
    fn hash_clear(&mut self) {
        self.address = None;
        self.output = None;
    }
}

impl HashClear for Transaction {
    fn hash_clear(&mut self) {
        // TODO: Implement hashclear for inputs
        for x in self.inputs.iter_mut() {
            x.hash_clear();
        }
        if let Some(s) = self.struct_metadata_opt() {
            s.hash_clear();
        }
    }
}

#[derive(PartialEq, PartialOrd, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddressBalance {
    pub address: String,
    pub rounded_balance: f64,
}


impl Transaction {

    pub fn combine_multisig_proofs(&mut self, other: &Transaction, address: &Address) -> RgResult<Transaction> {
        let mut updated = self.clone();
        for (idx, input) in updated.inputs.iter_mut().enumerate() {
            let other_input = other.inputs.get(idx).ok_msg("Missing input")?;
            for proof in other_input.proof.iter() {
                if input.proof.iter().any(|p| p.public_key == proof.public_key) {
                    continue;
                } else {
                    input.proof.push(proof.clone());
                }
            }
        }
        Ok(updated)
    }

    pub fn with_pow(mut self) -> RgResult<Self> {
        let hash = self.signable_hash();
        let proof = PoWProof::from_hash_sha3_256(&hash, 1)?;
        let opts = self.options.as_mut().expect("");
        opts.pow_proof = Some(proof);
        Ok(self)
    }

    pub fn signable_hash_or(&self) -> Hash {
        self.struct_metadata_opt_ref()
            .and_then(|s| s.signable_hash.clone()) // TODO: Change to as_ref() to prevent clone?
            .unwrap_or(self.signable_hash())
    }

    pub fn sponsored_time(&self) -> RgResult<i64> {
        let t = self.options()?.time_sponsor.safe_get_msg("Missing sponsored time")?.time;
        Ok(t)
    }

    pub fn transaction_type(&self) -> RgResult<structs::TransactionType> {
        let t = self.options()?.transaction_type;
        TransactionType::from_i32(t).ok_msg("Invalid transaction type")
    }

    pub fn is_metadata_or_obs(&self) -> bool {
        self.outputs.iter().all(|o| o.is_metadata() || o.observation().is_ok())
    }
    pub fn validate_network(&self, network: &NetworkEnvironment) -> RgResult<()> {
        let opts = self.options()?;
        let net = opts.network_type.safe_get_msg("Missing network type")?;
        let net = NetworkEnvironment::from_i32(net.clone());
        let net = net.safe_get_msg("Invalid network type")?;
        if net != network {
            Err(ErrorInfo::error_info("Invalid network type"))?
        }
        Ok(())
    }
    pub fn network(&self) -> RgResult<NetworkEnvironment> {
        let opts = self.options()?;
        let net = opts.network_type.safe_get_msg("Missing network type")?;
        let net = NetworkEnvironment::from_i32(net.clone());
        let net = net.safe_get_msg("Invalid network type")?;
        Ok(net.clone())
    }

    pub fn is_test(&self) -> bool {
        self.options.as_ref().and_then(|o| o.is_test).unwrap_or(false)
    }
    // TODO: Validator should ensure UtxoId only used once
    // Lets rename all UtxoId just utxoid
    pub fn input_of(&self, f: &UtxoId) -> Option<&Input> {
        self.inputs.iter().find(|i| i.utxo_id.as_ref().filter(|&u| u == f).is_some())
    }

    pub fn is_swap(&self) -> bool {
        self.outputs.iter().filter(|o| o.is_swap()).count() > 0
    }

    pub fn is_swap_fulfillment(&self) -> bool {
        self.outputs.iter().filter(|o| o.is_swap_fulfillment()).count() > 0
    }

    pub fn swap_fulfillment(&self) -> Option<&SwapFulfillment> {
        self.outputs.iter().filter_map(|o| o.swap_fulfillment()).next()
    }

    pub fn swap_fulfillment_amount_and_destination_and_origin(&self) -> Option<(&SwapFulfillment, &CurrencyAmount, &Address, Address)> {
        self.first_input_address().and_then(|origin|
            self.outputs.iter().filter_map(|o| {
                o.swap_fulfillment().and_then(|f| {
                    o.opt_amount_typed_ref().and_then(|a|
                        o.address.as_ref().map(|addr| (f, a, addr, origin.clone()))
                    )
                })
            }).next()
        )
    }

    pub fn output_data(&self) -> impl Iterator<Item=&StandardData> {
        self.outputs.iter().filter_map(|o| o.data.as_ref())
    }

    pub fn output_request(&self) -> impl Iterator<Item=&StandardRequest> {
        self.output_data().map(|d| d.standard_request.as_ref()).flatten()
    }

    pub fn output_response(&self) -> impl Iterator<Item=&StandardResponse> {
        self.output_data().map(|d| d.standard_response.as_ref()).flatten()
    }

    pub fn swap_request(&self) -> Option<&SwapRequest> {
        self.output_request().filter_map(|d| d.swap_request.as_ref()).next()
    }

    pub fn swap_request_and_amount_and_party_address(&self) -> Option<(&SwapRequest, &CurrencyAmount, &Address)> {
        self.outputs.iter().filter_map(|o| {
            o.opt_amount_typed_ref()
                .and_then(|a|
                    o.address.as_ref()
                        .and_then(|addr| o.swap_request()
                            .map(|r| (r, a, addr))
                ))
        }).next()
    }

    pub fn swap_destination(&self) -> Option<&Address> {
        self.swap_request().and_then(|r| r.destination.as_ref())
    }

    pub fn stake_request(&self) -> Option<&StakeRequest> {
        self.output_request().filter_map(|d| d.stake_request.as_ref()).next()
    }

    pub fn stake_deposit_request(&self) -> Option<&StakeDeposit> {
        self.stake_request().and_then(|d| d.deposit.as_ref())
    }

    pub fn stake_withdrawal_request(&self) -> Option<&StakeWithdrawal> {
        self.stake_request().and_then(|d| d.withdrawal.as_ref())
    }

    pub fn stake_withdrawal_fulfillments(&self) -> impl Iterator<Item =&structs::StakeWithdrawalFulfillment> {
        self.output_response().flat_map(|r| r.stake_withdrawal_fulfillment.as_ref())
    }

    pub fn stake_deposit_destination(&self) -> Option<&Address> {
        self.stake_deposit_request()
            .and_then(|d| d.deposit.as_ref())
            .and_then(|d| d.address.as_ref())
    }

    pub fn stake_destination(&self) -> Option<&Address> {
        self.stake_deposit_destination().or(self.stake_withdrawal_destination())
    }

    pub fn stake_withdrawal_destination(&self) -> Option<&Address> {
        self.stake_withdrawal_request()
            .and_then(|d| d.destination.as_ref())
    }

    pub fn swap_destination_currency(&self) -> Option<SupportedCurrency> {
        self.swap_destination().map(|a| a.currency_or())
    }

    pub fn external_destination_currency(&self) -> Option<SupportedCurrency> {
        self.swap_destination().or(self.stake_destination())
            .map(|d| d.clone().mark_external().clone())
            .map(|a| a.currency_or())
    }

    pub fn is_stake(&self) -> bool {
        self.outputs.iter().filter(|o| o.is_stake()).count() > 0
    }

    pub fn has_portfolio_request(&self) -> bool {
        self.portfolio_request().is_some()
    }

    pub fn portfolio_request(&self) -> Option<&PortfolioRequest> {
        self.outputs.iter()
            .filter_map(|o| o.request())
            .filter_map(|r| r.portfolio_request.as_ref())
            .next()
    }


    pub fn is_metadata(&self) -> bool {
        self.outputs.iter().filter(|o| o.is_metadata()).count() > 0
    }
    pub fn is_request(&self) -> bool {
        self.outputs.iter().filter(|o| o.is_request()).count() > 0
    }
    pub fn is_deploy(&self) -> bool {
        self.outputs.iter().filter(|o| o.is_deploy()).count() > 0
    }

    pub fn observation_output_index(&self) -> RgResult<i64> {
        self.outputs.iter().enumerate().find(|(_i, o)| o.observation().is_ok()
        ).map(|o| o.0 as i64).ok_or(error_info("Missing observation output"))
    }

    pub fn observation_as_utxo_id(&self) -> RgResult<UtxoId> {
        let o = self.observation_output_index();
        Ok(UtxoId {
            transaction_hash: Some(self.hash_or()),
            output_index: o?,
        })
    }

    pub fn observation_output(&self) -> RgResult<&Output> {
        let o = self.observation_output_index()?;
        let option = self.outputs.get(o as usize);
        option.safe_get_msg("Missing observation output").cloned()
    }

    pub fn observation_output_as(&self) -> RgResult<UtxoEntry> {
        let idx = self.observation_output_index()?;
        let o = self.observation_output()?;
        let u = o.utxo_entry(&self.hash_or(), idx as i64, self.time()?.clone());
        Ok(u)
    }

    pub fn observation(&self) -> RgResult<&Observation> {
        let mut map = self.outputs.iter().filter_map(|o| o.observation().ok());
        let option = map.next();
        option.ok_or(error_info("Missing observation"))
    }

    pub fn observation_proof(&self) -> RgResult<&Proof> {
        let o = self.observation()?;
        let p = o.parent_id.safe_get_msg("Missing parent id")?;
        let input_opt = self.input_of(&p);
        let input = input_opt.safe_get_msg("Missing input")?;
        let proof = input.proof.get(0);
        let proof_act = proof.safe_get_msg("Missing input proof")?;
        Ok(*proof_act)
    }
    pub fn observation_public_key(&self) -> RgResult<&PublicKey> {
        let proof_act = self.observation_proof()?;
        let pk = proof_act.public_key.safe_get_msg("Missing public key")?;
        Ok(pk)
    }

    pub fn build_observation_proofs(&self) -> RgResult<Vec<ObservationProof>> {
        let h = self.hash_or();
        let p = self.observation_proof()?;
        let o = self.observation()?;
        Ok(o.build_observation_proofs(&h, &p))
    }

    pub fn with_hashes(&mut self) -> &mut Self {
        self.with_hash();
        self.with_signable_hash().expect("signable hash");
        self
    }

    pub fn with_signable_hash(&mut self) -> Result<&mut Self, ErrorInfo> {
        self.struct_metadata()?.signable_hash = Some(self.signable_hash());
        Ok(self)
    }

    pub fn add_proof_per_input(&mut self, proof: &Proof) -> &Transaction {
        for i in self.inputs.iter_mut() {
            i.proof.push(proof.clone());
        }
        self
    }

    pub fn utxo_inputs(&self) -> impl Iterator<Item = &UtxoId> {
        self.inputs.iter().filter_map(|i| i.utxo_id.as_ref())
    }

    // This doesn't really need to return an error?
    pub fn fixed_utxo_ids_of_inputs(&self) -> Result<Vec<UtxoId>, ErrorInfo> {
        let mut utxo_ids = Vec::new();
        for input in &self.inputs {
            if let Some(f) = &input.utxo_id {
                utxo_ids.push(f.clone());
            }
        }
        Ok(utxo_ids)
    }

    // This doesn't really need to return an error?
    pub fn input_utxo_ids(&self) -> impl Iterator<Item = &UtxoId> {
        self.inputs.iter().flat_map(|i| i.utxo_id.as_ref())
    }

    pub fn output_amounts(&self) -> impl Iterator<Item=AddressBalance> + '_ {
        self.outputs
            .iter()
            .flat_map(|o| o.address.as_ref()
                .and_then(|a| a.render_string().ok())
                .and_then(|a| o.opt_amount_typed().map(|aa|
                    AddressBalance {
                        address: a,
                        rounded_balance: aa.to_fractional()
                    })
                )
            )
    }
    pub fn output_amount_total(&self) -> CurrencyAmount {
        self.output_amounts_opt().cloned().sum::<CurrencyAmount>()
    }

    pub fn output_amounts_opt(&self) -> impl Iterator<Item = &CurrencyAmount> {
        self.outputs
            .iter()
            .flat_map(|o| o.data.as_ref().and_then(|d| d.amount.as_ref()))
    }

    pub fn output_address_amounts_opt(&self) -> impl Iterator<Item = (&Address, CurrencyAmount)> {
        self.outputs
            .iter()
            .flat_map(|o| o.opt_amount_typed()
                .and_then(|a| o.address.as_ref().map(|addr| (addr, a)))
            )
    }

    pub fn output_amount_map<'a>(&'a self) -> HashMap<&'a Address, i64> {
        self.outputs
            .iter()
            .filter_map(|o| {
                // Now working entirely with references, avoiding cloning.
                // This assumes `o.address` is an Option<&Address> and `o.opt_amount()`
                // returns an Option<i64>.
                o.address.as_ref().and_then(|ad| o.opt_amount().map(|a| (ad, a)))
            })
            .fold(HashMap::new(), |mut acc: HashMap<&'a Address, i64>, (ad, a)| {
                *acc.entry(&ad).or_insert(0) += a;
                acc
            })
    }

    // TODO: Fix this, it's wrong
    pub fn non_remainder_amount_rdg(&self) -> i64 {
        let inputs = self.input_address_set();
        self.outputs.iter().filter_map(|o| {
            o.address.as_ref()
                .filter(|&a| !inputs.contains(a))
                .and_then(|_| o.opt_amount())}
        ).sum::<i64>()
    }
    pub fn non_remainder_amount_rdg_typed(&self) -> CurrencyAmount {
        let inputs = self.input_address_set();
        self.outputs.iter().filter_map(|o| {
            o.address.as_ref()
                .filter(|&a| !inputs.contains(a))
                .and_then(|_| o.opt_amount_typed())}
        ).sum::<CurrencyAmount>()
    }

    // TODO: Return as currency amount with plus operators, also maybe make optional?
    pub fn remainder_amount(&self) -> i64 {
        let inputs = self.input_address_set();
        self.outputs.iter().filter_map(|o| {
            o.address.as_ref()
                .filter(|&a| inputs.contains(a))
                .and_then(|_| o.opt_amount())}
        ).sum::<i64>()
    }
    pub fn fee_amount(&self) -> i64 {
        self.outputs.iter()
            .filter(|o| o.is_fee())
            .flat_map(|o| o.opt_amount())
            .sum::<i64>()
    }

    pub fn first_output_external_txid(&self) -> Option<&ExternalTransactionId> {
        self.output_external_txids().next()
    }
    pub fn output_external_txids(&self) -> impl Iterator<Item = &ExternalTransactionId> {
        self.output_response()
            .filter_map(|r| r.swap_fulfillment.as_ref())
            .filter_map(|d| d.external_transaction_id.as_ref())
    }

    pub fn output_rdg_amount_of(&self, address: &Address) -> i64 {
        self.outputs
            .iter()
            .filter_map(|o| o.address.as_ref().filter(|&a| a == address)
                .and_then(|_| o.opt_amount_typed()))
            .filter(|a| a.currency_or() == SupportedCurrency::Redgold)
            .map(|a| a.amount)
            .sum::<i64>()
    }

    pub fn output_of(&self, address: &Address) -> Vec<&structs::Output> {
        self.outputs
            .iter()
            .filter_map(|o| o.address.as_ref().filter(|&a| a == address).map(|_| o))
            .collect_vec()
    }

    pub fn output_of_with_index(&self, address: &Address) -> Vec<(usize, &structs::Output)> {
        self.outputs
            .iter()
            .enumerate()
            .filter_map(|(index, o)| o.address.as_ref().filter(|&a| a == address).map(|_| (index, o)))
            .collect_vec()
    }

    pub fn first_peer_utxo(&self) -> RgResult<UtxoEntry> {
        let vec = self.utxo_outputs()?;
        let option = vec.iter()
            .filter(|f| f.output.as_ref().filter(|o| o.is_peer_data()).is_some())
            .next();
        let x = option.ok_or(error_info("Missing peer utxo"))?.clone();
        Ok(x)
    }

    pub fn output_swap_amount_of(&self, address: &Address) -> i64 {
        self.outputs
            .iter()
            .filter_map(|o| {
                if o.is_swap() {
                    o.address.as_ref()
                        .filter(|&a| a == address)
                        .and_then(|_| o.opt_amount())
                } else {
                    None
                }
            }).sum::<i64>()
    }

    pub fn has_swap_to(&self, address: &Address) -> bool {
        self.output_swap_amount_of(address) > 0
    }

    pub fn output_bitcoin_address_of(&self, address: &Address) -> Option<&Address> {
        address.render_string().ok().and_then(|s| {
            self.outputs
                .iter()
                .filter(|o| {
                    o.is_swap()
                        .then(|| o.address.as_ref())
                        .flatten()
                        .and_then(|a| a.render_string().ok())
                        .filter(|a| {
                            a == &s
                        })
                        .is_some()
                })
                .filter_map(|o| o.data.as_ref().and_then(|d| d.address.as_ref()))
                .next()
        })
    }

    pub fn output_index(&self, output: &Output) -> RgResult<i64> {
        let index = self.outputs.iter().position(|o| o == output);
        index.safe_get_msg("Missing output index").map(|i| i.clone() as i64)
    }

    pub fn utxo_id_at(&self, index: usize) -> RgResult<UtxoId> {
        let hash = self.hash_or();
        let output = self.outputs.get(index);
        output.safe_get_msg("Missing output")?;
        let utxo_id = UtxoId::new(&hash, index as i64);
        Ok(utxo_id)
    }

    pub fn liquidity_of(&self, a: &Address) -> Vec<(UtxoId, &StakeRequest)> {
        self.output_of_with_index(a)
            .iter()
            .flat_map(|(u, o)|
                self.utxo_id_at(*u).ok().and_then(|utxo_id|
                    o.data.as_ref()
                        .and_then(|d| d.standard_request.as_ref())
                        .and_then(|d| d.stake_request.as_ref())
                        .map(|l| (utxo_id, l))
            ))
            .collect_vec()
    }
    pub fn stake_requests(&self) -> Vec<(UtxoId, &StakeRequest)> {
        self.outputs
            .iter()
            .enumerate()
            .flat_map(|(u, o)|
                self.utxo_id_at(u).ok().and_then(|utxo_id|
                    o.stake_request()
                        .map(|l| (utxo_id, l))
            ))
            .collect_vec()
    }

    pub fn total_output_amount(&self) -> i64 {
        let mut total = 0;
        for o in &self.outputs {
            if let Some(a) = o.opt_amount() {
                total += a
            }
        }
        total
    }

    pub fn floating_inputs(&self) -> impl Iterator<Item = &FloatingUtxoId> {
        self.inputs.iter().filter_map(|i| i.floating_utxo_id.as_ref())
    }

    pub fn total_input_amount(&self) -> i64 {
        self.inputs.iter()
            .filter_map(|i| i.output.as_ref())
            .filter_map(|o| o.opt_amount())
            .sum()
    }

    pub fn total_output_amount_float(&self) -> f64 {
        CurrencyAmount::from(
        self.total_output_amount()
        ).to_fractional()
    }

    pub fn output_amounts_by_product(&self) -> HashMap<ProductId, CurrencyAmount> {
        let mut map = HashMap::new();
        for output in &self.outputs {
            if let Some(product_id) = output.product_id.as_ref() {
                if let Some(a) = output.opt_amount() {
                    let aa = map.get(product_id).map(|x: &CurrencyAmount| x.amount + a).unwrap_or(a);
                    map.insert(product_id.clone(), CurrencyAmount::from(aa));
                }
            }
        }
        map
    }

    #[allow(dead_code)]
    fn clear_input_proofs(&mut self) {
        for i in 0..self.inputs.len() {
            self.inputs.get_mut(i).unwrap().proof.clear();
        }
    }

    #[allow(dead_code)]
    fn clear_counter_party_proofs(&mut self) {
        for i in 0..self.outputs.len() {
            self.outputs
                .get_mut(i)
                .unwrap()
                .counter_party_proofs
                .clear();
        }
    }
    #[allow(dead_code)]
    fn clear_confirmation_proofs(&mut self) {
        for o in self.options.iter_mut() {
            for c in o.contract.iter_mut() {
                c.confirmation_proofs.clear()
            }
        }
    }
    #[allow(dead_code)]
    pub fn signable_hash(&self) -> Hash {
        let mut clone = self.clone();
        clone.clear_input_proofs();
        clone.clear_counter_party_proofs();
        clone.clear_confirmation_proofs();
        return clone.calculate_hash();
    }

    #[allow(dead_code)]
    pub fn signed_hash(&self) -> Hash {
        let mut clone = self.clone();
        clone.clear_counter_party_proofs();
        clone.clear_confirmation_proofs();
        return clone.calculate_hash();
    }


    #[allow(dead_code)]
    fn pre_counter_party_hash(&self) -> Hash {
        self.signed_hash()
    }

    #[allow(dead_code)]
    fn confirmation_hash(&self) -> Hash {
        let mut clone = self.clone();
        clone.clear_confirmation_proofs();
        return clone.calculate_hash();
    }

    pub fn to_utxo_entries(&self, time: u64) -> Vec<UtxoEntry> {
        return UtxoEntry::from_transaction(self, time as i64);
    }

    pub fn to_utxo_address(&self, address: &Address) -> Vec<UtxoEntry> {
        let mut res = vec![];
        let time = self.time();
        if let Ok(time) = time {
            for u in UtxoEntry::from_transaction(self, time.clone()) {
                if u.address() == Ok(address) {
                    res.push(u);
                }
            }
        }
        res
    }

    pub fn utxo_outputs(&self) -> RgResult<Vec<UtxoEntry>> {
        let t = self.time()?;
        return Ok(UtxoEntry::from_transaction(self, t.clone()));
    }

    pub fn output_utxo_ids(&self) -> impl Iterator<Item = &UtxoId>{
        self.outputs.iter().flat_map(|o| o.utxo_id.as_ref())
    }

    pub fn head_utxo(&self) -> RgResult<UtxoEntry> {
        let utxos = self.utxo_outputs()?;
        let head_utxo = utxos.get(0);
        let utxo = *head_utxo.safe_get_msg("Missing UTXO output for node metadata")?;
        Ok(utxo.clone())
    }

    pub fn nmd_utxo(&self) -> RgResult<UtxoEntry> {
        let utxos = self.utxo_outputs()?.iter().filter(|u|
            u.output.as_ref().map(|o| o.is_node_metadata()).unwrap_or(false)
        ).cloned().collect_vec();
        let head_utxo = utxos.get(0);
        let utxo = *head_utxo.safe_get_msg("Missing UTXO output for node metadata")?;
        Ok(utxo.clone())
    }

    pub fn height(&self) -> RgResult<i64> {
        let h = self.options.as_ref()
            .and_then(|o| o.data.as_ref())
            .and_then(|d| d.standard_data.as_ref())
            .and_then(|s| s.height);
        h.safe_get_msg("Missing height").cloned()
    }

    pub fn options(&self) -> RgResult<&TransactionOptions> {
        self.options.safe_get_msg("Missing options")
    }
    pub fn salt(&self) -> RgResult<i64> {
        let s = self.options()?.salt;
        s.safe_get_msg("Missing salt").cloned()
    }

    // pub fn to_utxo_outputs_as_inputs(&self, time: u64) -> Vec<Input> {
    //     return UtxoEntry::from_transaction(self, time as i64)
    //         .iter()
    //         .map(|u| u.to_input())
    //         .collect::<Vec<Input>>();
    // }
    //
    // pub fn to_utxo_input_ids(&self) -> Vec<Vec<u8>> {
    //     return UtxoEntry::ids_from_transaction_inputs(self);
    // }
    //
    // pub fn to_utxo_output_ids(&self) -> Vec<Vec<u8>> {
    //     return UtxoEntry::ids_from_transaction_outputs(self);
    // }

    // pub fn currency_contract_hash() -> Vec<u8> {
    //     dhash_str("Redgold_currency_contract").to_vec()
    // }

    // This function seems to halt with a bad amoubnt calll

    pub fn peer_data(&self) -> Result<PeerMetadata, ErrorInfo> {
        let mut res = vec![];
        for o in &self.outputs {
            if let Some(data) = &o.data {
                if let Some(d) = &data.peer_data {
                    res.push(d.clone());
                }
            }
        }
        if res.len() == 1 {
            return Ok(res[0].clone());
        } else {
            return Err(ErrorInfo::error_info("Missing peer data in transaction"));
        }
    }

    pub fn node_metadata(&self) -> Result<NodeMetadata, ErrorInfo> {
        let mut res = vec![];
        for o in &self.outputs {
            if let Some(data) = &o.data {
                if let Some(d) = &data.node_metadata {
                    res.push(d.clone());
                }
            }
        }
        if res.len() == 1 {
            return Ok(res[0].clone());
        } else {
            return Err(ErrorInfo::error_info("Missing node metadata in transaction"));
        }
    }

    #[deprecated]
    pub fn addresses(&self) -> Vec<Address> {
        self.outputs.iter().filter_map(|o| o.address.clone()).collect_vec()
    }

    #[deprecated]
    pub fn input_addresses(&self) -> Vec<Address> {
        self.inputs.iter().filter_map(|o| o.address().ok()).collect_vec()
    }

    pub fn input_address_descriptor_address_or_public_key(&self) -> Vec<Address> {
        self.inputs.iter().filter_map(|i| {
            if let Some(d) = i.address_descriptor.as_ref() {
                Some(d.to_address())
            } else {
                i.proof.get(0)
                    .and_then(|p| p.public_key.as_ref())
                    .and_then(|pk| pk.address().ok())
            }
        }).collect_vec()
    }

    #[deprecated]
    pub fn input_address_set(&self) -> HashSet<Address> {
        self.inputs.iter().filter_map(|o| o.address().ok()).collect()
    }

    #[deprecated]
    pub fn first_input_address(&self) -> Option<Address> {
        self.inputs.iter().flat_map(|o| o.address().ok()).next()
    }

    #[deprecated]
    pub fn first_input_proof_public_key(&self) -> Option<&structs::PublicKey> {
        self.inputs.first()
            .and_then(|o| o.proof.get(0))
            .and_then(|o| o.public_key.as_ref())
    }

    pub fn first_output_non_input_or_fee(&self) -> Option<&Output> {
        let input_addrs = self.input_addresses();
        self.outputs.iter()
            .filter(|o| o.output_type != Some(OutputType::Fee as i32))
            .filter(|o| o.address.as_ref().map(|a| !input_addrs.contains(a)).unwrap_or(true))
            .next()
    }

    pub fn has_input_address_as_output_address(&self) -> bool {
        let input_addrs = self.input_addresses();
        self.outputs.iter()
            .filter(|o| o.address.as_ref().map(|a| input_addrs.contains(a)).unwrap_or(false))
            .next()
            .is_some()
    }

    pub fn is_outgoing(&self) -> bool {
        self.has_input_address_as_output_address()
    }

    pub fn first_output_address_non_input_or_fee(&self) -> Option<Address> {
        self.first_output_non_input_or_fee()
            .and_then(|o| o.address.clone())
    }

    pub fn first_output_amount(&self) -> Option<f64> {
        self.first_output_amount_typed().map(|o| o.to_fractional())
    }

    pub fn first_output_amount_i64(&self) -> Option<i64> {
        self.first_output_non_input_or_fee().and_then(|o| o.opt_amount())
    }

    pub fn first_output_amount_typed(&self) -> Option<CurrencyAmount> {
        self.first_output_non_input_or_fee().and_then(|o| o.opt_amount_typed())
    }

    pub fn first_contract_type(&self) -> Option<StandardContractType> {
        self.outputs.iter()
            .flat_map(|o| o.contract.as_ref())
            .flat_map(|c| c.standard_contract_type.as_ref())
            .flat_map(|c| StandardContractType::from_i32(c.clone()))
            .next()
    }

}

// #[derive(Clone)]
// pub struct LiquidityInfo


// TODO: ove into standard data
pub fn amount_data(amount: u64) -> Option<StandardData> {
    StandardData::amount_data(amount)
}

impl StandardData {

    pub fn observation(o: Observation) -> Self {
        let mut sd = Self::default();
        sd.observation = Some(o);
        sd
    }

    pub fn empty() -> Self {
        Self::default()
    }
    pub fn peer_data(pd: PeerMetadata) -> Option<Self> {
        let mut mt = Self::empty();
        mt.peer_data = Some(pd);
        Some(mt)
    }
    pub fn amount_data(amount: u64) -> Option<Self> {
        let mut mt = Self::empty();
        mt.amount = Some(CurrencyAmount::from(amount as i64));
        Some(mt)
    }


}

impl TypedValue {
    pub fn bytes(bytes: &Vec<u8>) -> Self {
        let mut s = Self::default();
        s.bytes_value = bytes_data(bytes.clone());
        s
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionMaybeError {
    pub transaction: Transaction,
    pub error: Option<ErrorInfo>,
}

impl TransactionMaybeError {
    pub fn new(transaction: Transaction) -> Self {
        Self {
            transaction,
            error: None,
        }
    }
    pub fn new_error(transaction: Transaction, error: ErrorInfo) -> Self {
        Self {
            transaction,
            error: Some(error),
        }
    }
}