use std::collections::HashMap;
use std::iter::FilterMap;
use std::slice::Iter;
use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY, MAX_INPUTS_OUTPUTS};
use crate::structs::{Address, BytesData, Error as RGError, ErrorInfo, FixedUtxoId, FloatingUtxoId, Hash, Input, NodeMetadata, ProductId, Proof, StandardData, StructMetadata, Transaction, CurrencyAmount, TypedValue, UtxoEntry};
use crate::utxo_id::UtxoId;
use crate::{bytes_data, error_code, error_info, error_message, ErrorInfoContext, HashClear, PeerData, ProtoHashable, RgResult, SafeBytesAccess, SafeOption, structs, WithMetadataHashable, WithMetadataHashableFields};
use itertools::Itertools;
use crate::transaction_builder::TransactionBuilder;

pub const MAX_TRANSACTION_MESSAGE_SIZE: usize = 40;


impl FixedUtxoId {
    pub fn format_str(&self) -> String {
        format!("FixedUtxoId: {} output: {}", self.transaction_hash.clone().expect("").hex(), self.output_index)
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
        for mut x in self.inputs.iter_mut() {
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


    pub fn fixed_utxo_ids_of_inputs(&self) -> Result<Vec<FixedUtxoId>, ErrorInfo> {
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

    pub fn output_bitcoin_address_of(&self, address: &Address) -> Option<&String> {
        self.outputs
            .iter()
            .filter(|o| {
                if o.is_swap() {
                    o.address.as_ref()
                        .filter(|&a| a == address).is_some()
                } else {
                    false
                }
            })
            .filter_map(|o| o.data.as_ref().and_then(|d| d.bitcoin_address.as_ref()))
            .next()
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

    pub fn peer_data(&self) -> Result<PeerData, ErrorInfo> {
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
            return Err(ErrorInfo::error_info("Missing peer data in transaction"));
        }
    }

    pub fn addresses(&self) -> Vec<Address> {
        self.outputs.iter().filter_map(|o| o.address.clone()).collect_vec()
    }

    pub fn input_addresses(&self) -> Vec<Address> {
        self.inputs.iter().filter_map(|o| o.address().ok()).collect_vec()
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

impl CurrencyAmount {
    pub fn from_fractional(a: f64) -> Result<Self, ErrorInfo> {
        if a <= 0 as f64 {
            Err(ErrorInfo::error_info("Invalid negative or zero transaction amount"))?
        }
        if a > DECIMAL_MULTIPLIER as f64 {
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

    pub fn empty() -> Self {
        Self::default()
    }
    pub fn peer_data(pd: PeerData) -> Option<Self> {
        let mut mt = Self::empty();
        mt.peer_data = Some(pd);
        Some(mt)
    }
    pub fn amount_data(amount: u64) -> Option<Self> {
        let mut mt = Self::empty();
        mt.amount = Some(amount as i64);
        Some(mt)
    }

    pub fn bytes_data(bytes: &Vec<u8>) -> Self {
        let mut mt = Self::empty();
        mt.typed_value = Some(TypedValue::bytes(bytes));
        mt
    }

    pub fn bytes(&self) -> Option<&BytesData> {
        self.typed_value.as_ref().and_then(|t| t.bytes_value.as_ref())
    }

}

impl TypedValue {
    pub fn bytes(bytes: &Vec<u8>) -> Self {
        let mut s = Self::default();
        s.bytes_value = bytes_data(bytes.clone());
        s
    }
}