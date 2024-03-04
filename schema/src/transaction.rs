use std::collections::{HashMap, HashSet};
use std::iter::FilterMap;
use std::slice::Iter;
use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY, MAX_INPUTS_OUTPUTS};
use crate::structs::{Address, BytesData, Error as RGError, ErrorInfo, UtxoId, FloatingUtxoId, Hash, Input, NodeMetadata, ProductId, Proof, StandardData, StructMetadata, Transaction, CurrencyAmount, TypedValue, UtxoEntry, Observation, PublicKey, TransactionOptions, Output, ObservationProof, HashType, LiquidityRequest, NetworkEnvironment, ExternalTransactionId};
use crate::utxo_id::OldUtxoId;
use crate::{bytes_data, error_code, error_info, error_message, ErrorInfoContext, HashClear, PeerMetadata, ProtoHashable, RgResult, SafeBytesAccess, SafeOption, struct_metadata_new, structs, WithMetadataHashable, WithMetadataHashableFields};
use itertools::Itertools;
use rand::Rng;

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
    pub fn from(id: Vec<u8>) -> UtxoId {
        let len = id.len();
        let split = len - 8;
        let mut output_index_array = [0u8; 8];
        let output_id_vec = &id[split..len];
        output_index_array.clone_from_slice(&output_id_vec);
        let output_index = i64::from_le_bytes(output_index_array);
        let transaction_hash = id[0..split].to_vec();
        let transaction_hash = Some(Hash::new(transaction_hash));
        UtxoId {
            transaction_hash,
            output_index,
        }
    }
    pub fn utxo_id_vec(&self) -> Vec<u8> {
        let mut merged: Vec<u8> = vec![];
        merged.extend(self.transaction_hash.clone().expect("hash").vec());
        merged.extend(self.output_index.to_le_bytes().to_vec());
        merged
    }

    pub fn as_hash(&self) -> Hash {
        let mut h = Hash::new(self.utxo_id_vec());
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

impl HashClear for Transaction {
    fn hash_clear(&mut self) {
        // TODO: Implement hashclear for inputs
        for x in self.inputs.iter_mut() {
            x.output = None;
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

    pub fn new_blank() -> Self {
        let mut rng = rand::thread_rng();
        let mut tx = Self::default();
        tx.struct_metadata = struct_metadata_new();

        let mut opts = TransactionOptions::default();
        opts.salt = Some(rng.gen::<i64>());
        tx.options = Some(opts);
        tx
    }
    pub fn is_test(&self) -> bool {
        self.options.as_ref().and_then(|o| o.is_test).unwrap_or(false)
    }

    // TODO: Validator should ensure UtxoId only used once
    // Lets rename all UtxoId just utxoid
    pub fn input_of(&self, f: &UtxoId) -> Option<&Input> {
        self.inputs.iter().find(|i| i.utxo_id.as_ref().filter(|&u| u == f).is_some())
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
        Ok(proof_act.clone())
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


    // TODO: Is this wrong definition here?
    pub fn iter_utxo_outputs(&self) -> Vec<(Vec<u8>, i64)> {
        self.outputs
            .iter()
            .enumerate()
            .map(|(x, _)| (self.hash_vec().clone(), x as i64))
            .collect_vec()
    }

    pub fn iter_utxo_inputs(&self) -> Vec<(Vec<u8>, i64)> {
        self.inputs
            .iter()
            .filter_map(|y| y.utxo_id.as_ref())
            .map(|y| {
                (
                    y.transaction_hash.safe_bytes().expect("a").clone(),
                    y.output_index,
                )
            })
            .collect_vec()
    }


    pub fn fixed_utxo_ids_of_inputs(&self) -> Result<Vec<UtxoId>, ErrorInfo> {
        let mut utxo_ids = Vec::new();
        for input in &self.inputs {
            if let Some(f) = &input.utxo_id {
                utxo_ids.push(f.clone());
            }
        }
        Ok(utxo_ids)
    }

    pub fn output_amounts(&self) -> Vec<AddressBalance> {
        self.outputs
            .iter()
            .map(|o|
            // makee hex trait for Vec<u8>
            AddressBalance{ address: hex::encode(o.address.safe_bytes().expect("bytes")),
                rounded_balance: o.rounded_amount()
            })
            .collect()
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

    pub fn non_remainder_amount(&self) -> i64 {
        let inputs = self.input_address_set();
        self.outputs.iter().filter_map(|o| {
            o.address.as_ref()
                .filter(|&a| !inputs.contains(a))
                .and_then(|_| o.opt_amount())}
        ).sum::<i64>()
    }

    pub fn first_output_external_txid(&self) -> Option<&ExternalTransactionId> {
        self.outputs
            .iter()
            .filter_map(|o| o.data.as_ref())
            .filter_map(|d| d.external_transaction_id.as_ref())
            .next()
    }
    pub fn output_external_txids(&self) -> impl Iterator<Item = &ExternalTransactionId> {
        self.outputs
            .iter()
            .filter_map(|o| o.data.as_ref())
            .filter_map(|d| d.external_transaction_id.as_ref())
    }

    pub fn output_amount_of(&self, address: &Address) -> i64 {
        self.outputs
            .iter()
            .filter_map(|o| o.address.as_ref().filter(|&a| a == address).and_then(|_| o.opt_amount()))
            .sum::<i64>()
    }

    pub fn output_of(&self, address: &Address) -> Vec<&structs::Output> {
        self.outputs
            .iter()
            .filter_map(|o| o.address.as_ref().filter(|&a| a == address).map(|_| o))
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

    pub fn liquidity_of(&self, a: &Address) -> Option<(CurrencyAmount, &LiquidityRequest)> {
        let output_contract = self.output_of(a).iter().next().cloned();
        let amount = output_contract
            .and_then(|o| o.data.as_ref().and_then(|d| d.amount.clone()));
        self.outputs.iter().filter(|o| o.is_liquidity()).next()
            .and_then(|o| o.data.as_ref().and_then(|d| d.liquidity_request.as_ref()))
            .and_then(|o| {
            amount.map(|a| {
                (a, o)
            })
        })
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
    fn counter_party_hash(&self) -> Hash {
        let mut clone = self.clone();
        clone.clear_counter_party_proofs();
        clone.clear_confirmation_proofs();
        return clone.calculate_hash();
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

    pub fn utxo_outputs(&self) -> RgResult<Vec<UtxoEntry>> {
        let t = self.time()?;
        return Ok(UtxoEntry::from_transaction(self, t.clone()));
    }

    pub fn head_utxo(&self) -> RgResult<UtxoEntry> {
        let utxos = self.utxo_outputs()?;
        let head_utxo = utxos.get(0);
        let utxo = head_utxo.safe_get_msg("Missing UTXO output for node metadata")?;
        Ok(utxo.clone().clone())
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

    pub fn addresses(&self) -> Vec<Address> {
        self.outputs.iter().filter_map(|o| o.address.clone()).collect_vec()
    }

    pub fn input_addresses(&self) -> Vec<Address> {
        self.inputs.iter().filter_map(|o| o.address().ok()).collect_vec()
    }

    pub fn input_address_set(&self) -> HashSet<Address> {
        self.inputs.iter().filter_map(|o| o.address().ok()).collect()
    }

    pub fn first_input_address(&self) -> Option<Address> {
        self.inputs.first().and_then(|o| o.address().ok())
    }

    pub fn first_input_proof_public_key(&self) -> Option<&structs::PublicKey> {
        self.inputs.first()
            .and_then(|o| o.proof.get(0))
            .and_then(|o| o.public_key.as_ref())
    }

    pub fn first_output_address(&self) -> Option<Address> {
        self.outputs.first().and_then(|o| o.address.clone())
    }

    pub fn first_output_amount(&self) -> Option<f64> {
        self.outputs.first().as_ref().map(|o| o.rounded_amount())
    }

}

// #[derive(Clone)]
// pub struct LiquidityInfo

impl CurrencyAmount {
    pub fn from_fractional(a: impl Into<f64>) -> Result<Self, ErrorInfo> {
        let a = a.into();
        if a <= 0f64 {
            Err(ErrorInfo::error_info("Invalid negative or zero transaction amount"))?
        }
        if a > MAX_COIN_SUPPLY as f64 {
            Err(ErrorInfo::error_info("Invalid transaction amount"))?
        }
        let amount = (a * (DECIMAL_MULTIPLIER as f64)) as i64;
        Ok(CurrencyAmount{
            amount
        })
    }
    pub fn to_fractional(&self) -> f64 {
        (self.amount as f64) / (DECIMAL_MULTIPLIER as f64)
    }

    pub fn to_rounded_int(&self) -> i64 {
        self.to_fractional() as i64
    }
    pub fn from(amount: i64) -> Self {
        Self {
            amount
        }
    }
    pub fn from_float_string(str: &String) -> Result<Self, ErrorInfo> {
        let amount = str.parse::<f64>()
            .error_info("Invalid transaction amount")?;
        Self::from_fractional(amount)
    }
}


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