use crate::address::address_data;
use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY, MAX_INPUTS_OUTPUTS};
use crate::structs::{Error as RGError, ErrorInfo, Hash, NodeMetadata, Output, Proof, StandardData, StructMetadata, Transaction, TransactionAmount, UtxoEntry};
use crate::utxo_id::UtxoId;
use crate::{error_message, struct_metadata, HashClear, ProtoHashable, SafeBytesAccess, WithMetadataHashable, WithMetadataHashableFields, constants, PeerData, Error};
use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, SecretKey, Signature};
use itertools::Itertools;

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
    fn set_hash(&mut self, hash: Hash) {
        self.hash = Some(hash);
    }

    fn stored_hash_opt(&self) -> Option<Hash> {
        self.hash.clone()
    }

    fn struct_metadata_opt(&self) -> Option<StructMetadata> {
        self.struct_metadata.clone()
    }
}

impl HashClear for Transaction {
    fn hash_clear(&mut self) {
        // TODO: Implement hashclear for inputs
        for mut x in self.inputs.iter_mut() {
            x.output = None;
        }
        self.hash = None;
    }
}

#[derive(PartialEq, PartialOrd, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddressBalance {
    pub address: String,
    pub rounded_balance: f64,
}

impl Transaction {
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
            .map(|y| {
                (
                    y.transaction_hash.safe_bytes().expect("a").clone(),
                    y.output_index,
                )
            })
            .collect_vec()
    }

    pub fn utxo_ids_of_inputs(&self) -> Result<Vec<UtxoId>, ErrorInfo> {
        let mut utxo_ids = Vec::new();
        for input in &self.inputs {
            utxo_ids.push(UtxoId {
                transaction_hash: input.transaction_hash.safe_bytes()?,
                output_index: input.output_index as i64,
            });
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

    pub fn total_output_amount(&self) -> i64 {
        let mut total = 0;
        for o in &self.outputs {
            if let Some(a) = o.opt_amount() {
                total += a
            }
        }
        total
    }

    pub fn verify_utxo_entry_proof(&self, utxo_entry: &UtxoEntry) -> Result<(), ErrorInfo> {
        let input = self
            .inputs
            .get(utxo_entry.output_index as usize)
            .ok_or(error_message(
                RGError::MissingInputs,
                format!("missing input index: {}", utxo_entry.output_index),
            ))?;
        Ok(Proof::verify_proofs(
            &input.proof,
            &utxo_entry.transaction_hash,
            &utxo_entry.address,
        )?)
    }

    pub fn prevalidate(&self) -> Result<(), RGError> {
        if self.inputs.is_empty() {
            return Err(RGError::MissingInputs);
        }
        if self.outputs.is_empty() {
            return Err(RGError::MissingOutputs);
        }
        // if self.fee < MIN_FEE_RAW {
        //     return Err(RGError::InsufficientFee);
        // }
        for input in self.inputs.iter() {
            if input.output_index > (MAX_INPUTS_OUTPUTS as i64) {
                return Err(RGError::InvalidAddressInputIndex);
            }
            // if input.transaction_hash.len() != 32 {
            //     // println!("transaction id len : {:?}", input.id.len());
            //     return Err(RGError::InvalidHashLength);
            // }
            if input.proof.is_empty() {
                return Err(RGError::MissingProof);
            }
        }

        for _output in self.outputs.iter() {
            // if output.address.len() != 20 {
            //     // println!("address id len : {:?}", output.address.len());
            //     return Err(RGError::InvalidHashLength);
            // }
        }
        return Ok(());
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
    fn signable_hash(&self) -> Hash {
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

    // TODO: TransactionBuilder
    pub fn new(
        source: &UtxoEntry,
        destination: &Vec<u8>,
        amount: u64,
        secret: &SecretKey,
        public: &PublicKey,
    ) -> Self {
        let mut input = source.to_input();
        let proof = Proof::new(
            &input.transaction_hash.as_ref().expect("hash"),
            secret,
            public,
        );
        input.proof.push(proof);
        let mut amount_actual = amount;
        if amount < (MAX_COIN_SUPPLY as u64) {
            amount_actual = amount * (DECIMAL_MULTIPLIER as u64);
        }
        let fee = 0 as u64; //MIN_FEE_RAW;
        amount_actual -= fee;
        let output = Output {
            address: address_data(destination.clone()),
            data: amount_data(amount_actual),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };

        let mut outputs: Vec<Output> = vec![output];
        // println!("{:?}, {:?}, {:?}", source.amount, amount_actual, fee);
        if source.amount() > (amount_actual + fee) {
            let remaining_amount = source.amount() - (amount_actual + fee);
            // TODO: Don't re-use keys in constructor
            let remainder = Output::from_public_amount(public, remaining_amount);
            outputs.push(remainder);
        }
        let mut tx = Self {
            inputs: vec![input],
            outputs,
            // TODO: Fix genesis cause this causes an issue
            struct_metadata: struct_metadata(0 as i64),
            options: None,
            hash: None,
        };

        tx.with_hash();

        tx
    }
    //
    // pub fn from(
    //     inputs: Vec<UtxoEntry>,
    //     amount_address: Vec<(u64, Vec<u8>)>,
    //     key_pairs: Vec<KeyPair>
    // ) -> Self {
    //     let mut input = source.to_input();
    //     let proof = Proof::new(&input.id, secret, public);
    //     input.proof.push(proof);
    //     let amount_actual = amount * PRIME_MULTIPLIER;
    //     let fee = MIN_FEE_RAW;
    //     let output = Output {
    //         address: destination.clone(),
    //         data: CurrencyData {
    //             amount: amount_actual,
    //         }
    //             .proto_serialize(),
    //         contract: Transaction::currency_contract_hash(),
    //         threshold: None,
    //         weights: vec![],
    //         product_id: None,
    //         counter_party_proofs: vec![],
    //     };
    //
    //     // println!("{:?}, {:?}, {:?}", source.amount, amount_actual, fee);
    //     let remaining_amount = source.amount() - amount_actual - fee;
    //     // TODO: Don't re-use keys in constructor
    //     let remainder = Output {
    //         address: address(public).to_vec(),
    //         data: CurrencyData {
    //             amount: remaining_amount,
    //         }
    //             .proto_serialize(),
    //         contract: Transaction::currency_contract_hash(),
    //         threshold: None,
    //         weights: vec![],
    //         product_id: None,
    //         counter_party_proofs: vec![],
    //     };
    //
    //     return Self {
    //         inputs: vec![input],
    //         outputs: vec![output, remainder],
    //         fee,
    //         version: None,
    //         finalize_window: None,
    //         confirmation_required: false,
    //         confirmation_proofs: vec![],
    //         message: None,
    //         pow_proof: None,
    //     };
    // }

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

}

impl TransactionAmount {
    pub fn from_fractional(a: f64) -> Result<Self, ErrorInfo> {
        if a <= 0 as f64 {
            Err(ErrorInfo::error_info("Invalid negative or zero transaction amount"))?
        }
        let amount = (a * (DECIMAL_MULTIPLIER as f64)) as i64;
        Ok(TransactionAmount{
            amount
        })
    }
    fn to_fractional(&self) -> f64 {
        (self.amount as f64) / (DECIMAL_MULTIPLIER as f64)
    }
}


// TODO: ove into standard data
pub fn amount_data(amount: u64) -> Option<StandardData> {
    StandardData::amount_data(amount)
}

impl StandardData {

    pub fn empty() -> Self {
        Self {
            amount: None,
            typed_value: None,
            typed_value_list: vec![],
            keyed_typed_value: None,
            keyed_typed_value_list: vec![],
            matrix_typed_value: None,
            matrix_typed_value_list: vec![],
            peer_data: None,
            node_metadata: None,
            dynamic_node_metadata: None,
            height: None,
            data_hash: None,
            hash: None,
        }
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
}