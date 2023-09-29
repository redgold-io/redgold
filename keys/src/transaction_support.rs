use std::collections::HashSet;
use bdk::bitcoin::secp256k1::rand;
use bdk::bitcoin::secp256k1::rand::Rng;
use bitcoin::secp256k1::{PublicKey, SecretKey};
use redgold_schema::{EasyJson, error_code, error_info, error_message, RgResult, SafeOption, struct_metadata_new, structs, WithMetadataHashable};
use redgold_schema::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY, MAX_INPUTS_OUTPUTS};
use redgold_schema::structs::{Address, ErrorInfo, Hash, Input, Proof, Transaction, CurrencyAmount, TransactionOptions, UtxoEntry, UtxoId};
use redgold_schema::transaction::MAX_TRANSACTION_MESSAGE_SIZE;
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::KeyPair;
use crate::proof_support::ProofSupport;

pub trait TransactionSupport {
    fn sign(&mut self, key_pair: &KeyPair) -> Result<Transaction, ErrorInfo>;
    // TODO: Move all of this to TransactionBuilder
    fn new(
        source: &UtxoEntry,
        destination: &Vec<u8>,
        amount: u64,
        secret: &SecretKey,
        public: &PublicKey,
    ) -> Self;
    fn verify_utxo_entry_proof(&self, utxo_entry: &UtxoEntry) -> Result<(), ErrorInfo>;
    fn validate(&self) -> RgResult<()>;
    fn prevalidate(&self) -> Result<(), ErrorInfo>;
}

impl TransactionSupport for Transaction {
    fn sign(&mut self, key_pair: &KeyPair) -> RgResult<Transaction> {
        let hash = self.signable_hash();
        let addr = key_pair.address_typed();
        let mut signed = false;
        for i in self.inputs.iter_mut() {
            if let Some(o) = i.output.as_ref() {
                let input_addr = o.address.safe_get_msg("Missing address on enriched output during signing")?;
                if &addr == input_addr {
                    let proof = Proof::from_keypair_hash(&hash, &key_pair);
                    i.proof.push(proof);
                    signed = true;
                }
            }
        }
        if !signed {
            return Err(error_info("Couldn't find appropriate input address to sign"));
        }
        let x = self.with_hash();
        x.struct_metadata.as_mut().expect("sm").signed_hash = Some(x.hash_or());
        Ok(x.clone())
    }

    // TODO: Move all of this to TransactionBuilder
    fn new(
        source: &UtxoEntry,
        destination: &Vec<u8>,
        amount: u64,
        secret: &SecretKey,
        public: &PublicKey,
    ) -> Self {

        let mut amount_actual = amount;
        if amount < (MAX_COIN_SUPPLY as u64) {
            amount_actual = amount * (DECIMAL_MULTIPLIER as u64);
        }
        let amount = CurrencyAmount::from(amount as i64);
        // let fee = 0 as u64; //MIN_FEE_RAW;
        // amount_actual -= fee;
        let destination = Address::from_bytes(destination.clone()).unwrap();
        let txb = TransactionBuilder::new()
            .with_utxo(&source).expect("")
            .with_output(&destination, &amount)
            .build().expect("")
            .sign(&KeyPair::new(&secret.clone(), &public.clone())).expect("");
        txb
    }


    fn verify_utxo_entry_proof(&self, utxo_entry: &UtxoEntry) -> Result<(), ErrorInfo> {
        let id = utxo_entry.utxo_id.safe_get_msg("Missing utxo id during verify_utxo_entry_proof")?;
        let input = self
            .inputs
            .get(id.output_index as usize)
            .ok_or(error_message(
                structs::Error::MissingInputs,
                format!("missing input index: {}", id.output_index),
            ))?;
        let address = utxo_entry.address()?;
        Ok(Proof::verify_proofs(
            &input.proof,
            &self.signable_hash(),
            address,
        )?)
    }

    fn validate(&self) -> RgResult<()> {
        self.prevalidate()?;
        for i in &self.inputs {
            i.verify(&self.signable_hash())?;
        }
        Ok(())
    }

    fn prevalidate(&self) -> RgResult<()> {

        let mut hs: HashSet<UtxoId> = HashSet::new();
        for i in &self.inputs {
            if let Some(f) = &i.utxo_id {
                if hs.contains(f) {
                    return Err(error_info("Duplicate input UTXO consumption"))?;
                }
                hs.insert(f.clone());
            }
        }
        if self.inputs.is_empty() {
            Err(error_code(structs::Error::MissingInputs))?;
        }
        if self.outputs.is_empty() {
            Err(error_code(structs::Error::MissingOutputs))?;
        }

        if let Some(o) = &self.options {
            if let Some(d) = &o.data {
                if let Some(m) = &d.message {
                    let i = m.len();
                    if i > MAX_TRANSACTION_MESSAGE_SIZE {
                        Err(
                            error_info(
                                format!(
                                    "Message length: {} too large, expected {}", i, MAX_TRANSACTION_MESSAGE_SIZE
                                )
                            )
                        )?;
                    }
                }
            }
        }

        // if self.fee < MIN_FEE_RAW {
        //     return Err(RGError::InsufficientFee);
        // }
        for input in self.inputs.iter() {
            if let Some(utxo) = input.utxo_id.as_ref() {
                if utxo.output_index > (MAX_INPUTS_OUTPUTS as i64) {
                    Err(error_code(structs::Error::InvalidAddressInputIndex))?;
                }
            }
            // if input.transaction_hash.len() != 32 {
            //     // println!("transaction id len : {:?}", input.id.len());
            //     error_code(RGError::InvalidHashLength);
            // }
            if input.proof.is_empty() {
                let floating_non_consume_input = input.utxo_id.is_none() && input.floating_utxo_id.is_some();
                if !floating_non_consume_input {
                    Err(error_message(structs::Error::MissingProof,
                                      format!("Input proof is missing on input {}", input.json_or()
                                      )))?;
                }
            }
            input.verify_signatures_only(&self.signable_hash())?;
        }

        for _output in self.outputs.iter() {
            // TODO: Reimplement dust separate from testing?
            // if output.address.len() != 20 {
            //     // println!("address id len : {:?}", output.address.len());
            //     return Err(RGError::InvalidHashLength);
            // }
            // if let Some(a) = _output.opt_amount() {
            //     if a < 10_000 {
            //         Err(error_info(format!("Insufficient amount output of {a}")))?;
            //     }
            // }
        }

        // TODO: Sum by product Id

        return Ok(());
    }



}

pub trait InputSupport {
    fn verify_proof(&self, address: &Address, hash: &Hash) -> Result<(), ErrorInfo>;
    // This does not verify the address on the prior output
    fn verify_signatures_only(&self, hash: &Hash) -> Result<(), ErrorInfo>;
    fn verify(&self, hash: &Hash) -> Result<(), ErrorInfo>;
}

impl InputSupport for Input {

    fn verify_proof(&self, address: &Address, hash: &Hash) -> Result<(), ErrorInfo> {
        Proof::verify_proofs(&self.proof, &hash, address)
    }

    // This does not verify the address on the prior output
    fn verify_signatures_only(&self, hash: &Hash) -> Result<(), ErrorInfo> {
        for proof in &self.proof {
            proof.verify(&hash)?
        }
        return Ok(());
    }

    fn verify(&self, hash: &Hash) -> Result<(), ErrorInfo> {
        let o = self.output.safe_get_msg("Missing enriched output on input for transaction verification")?;
        let address = o.address.safe_get_msg("Missing address on enriched output for transaction verification")?;
        self.verify_proof(&address, hash)?;
        return Ok(());
    }
}


pub trait TransactionBuilderSupport {
    fn new() -> Self;
}

impl TransactionBuilderSupport for TransactionBuilder {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            transaction: Transaction{
                inputs: vec![],
                outputs: vec![],
                struct_metadata: struct_metadata_new(),
                options: Some(TransactionOptions{
                    salt: Some(rng.gen::<i64>()),
                    // TODO: None here or with setter?
                    network_type: None,
                    key_value_options: vec![],
                    data: None,
                    contract: None,
                    offline_time_sponsor: None,
                }),
            },
            utxos: vec![],
            used_utxos: vec![],
        }
    }
}