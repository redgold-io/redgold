use itertools::Itertools;
use redgold_schema::{error_message, RgResult, SafeOption, structs};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, DebugSerChange, DebugSerChange2, ErrorInfo, Hash, Input, NetworkEnvironment, Output, Proof, PublicKey, SupportedCurrency, TimeSponsor, Transaction, TransactionOptions, UtxoEntry};
use crate::KeyPair;
use crate::proof_support::{ProofSupport, PublicKeySupport};




use redgold_schema::util::times::current_time_millis;
use crate::address_external::ToBitcoinAddress;

pub trait TransactionSupport {
    fn time_sponsor(&mut self, key_pair: KeyPair) -> RgResult<Transaction>;
    fn sign(&mut self, key_pair: &KeyPair) -> Result<Transaction, ErrorInfo>;
    // TODO: Move all of this to TransactionBuilder
    fn inputs_match_pk_address(&self, other_address: &Address) -> bool;
    fn first_input_address_to_btc_address(&self, network: &NetworkEnvironment) -> Option<String>;
    fn outputs_of_pk(&self, pk: &PublicKey) -> RgResult<impl Iterator<Item=&Output>>;
    fn output_rdg_amount_of_pk(&self, pk: &PublicKey) -> RgResult<CurrencyAmount>;
    fn outputs_of_exclude_pk(&self, pk: &PublicKey) -> RgResult<impl Iterator<Item=&Output>>;
    fn output_rdg_amount_of_exclude_pk(&self, pk: &PublicKey) -> RgResult<CurrencyAmount>;
}

#[test]
fn proto_ser_remove_opt_change() {
    let ser = DebugSerChange{
        field1: "asdf".to_string(),
        field2: None
    }.proto_serialize();
    let sec = DebugSerChange2::proto_deserialize(ser).expect("deser");
    assert_eq!(sec.field1, "asdf".to_string());
}

impl TransactionSupport for Transaction {
    fn time_sponsor(&mut self, key_pair: KeyPair) -> RgResult<Transaction> {
        let x = self.with_hash();
        let hash = x.hash_or();
        let mut options = TransactionOptions::default();
        let opts = x.options.as_mut().unwrap_or(&mut options);
        if opts.time_sponsor.is_none() {
            let time = current_time_millis();
            let proof = Proof::from_keypair_hash(&hash, &key_pair);
            let mut ot = TimeSponsor::default();
            ot.proof = Some(proof);
            ot.time = time;
            opts.time_sponsor = Some(ot);
            x.with_hash();
        }
        Ok(x.clone())
    }

    // Simple signing function, this won't work for multi-sig, construct separately
    fn sign(&mut self, key_pair: &KeyPair) -> RgResult<Transaction> {
        let pk = key_pair.public_key();
        let hash = self.signable_hash();
        let all_key_pair_addresses = pk.to_all_addresses()?;
        let mut signed = false;
        for i in self.inputs.iter_mut() {
            if let Some(o) = i.output.as_ref() {
                if i.proof.iter().flat_map(|p| p.public_key.as_ref()).contains(&pk) {
                    // info!("Already signed");
                    continue;
                }
                let input_addr = o.address.safe_get_msg("Missing address on enriched output during signing")?;
                if all_key_pair_addresses.contains(input_addr) {
                    let proof = Proof::from_keypair_hash(&hash, &key_pair);
                    i.proof.push(proof);
                    signed = true;
                }
            }
        }
        // if !signed {
        //     return Err(error_info("Couldn't find appropriate input address to sign"));
        // }
        let x = self.with_hash();
        x.struct_metadata.as_mut().expect("sm").signed_hash = Some(x.hash_or());
        Ok(x.clone())
    }

    fn inputs_match_pk_address(&self, other_address: &Address) -> bool {
        self.inputs.iter()
            .flat_map(|i| -> &Vec<Proof> { i.proof.as_ref()})
            .filter_map(|p: &structs::Proof| p.public_key.as_ref())
            .filter_map(|pk| pk.to_all_addresses().ok())
            .flatten()
            .filter(|a| a == other_address)
            .count() > 0
    }

    fn outputs_of_pk(&self, pk: &PublicKey) -> RgResult<impl Iterator<Item=&Output>> {
        let all = pk.to_all_addresses()?;
        Ok(self.outputs.iter()
            .filter(move |o| o.address.as_ref().filter(|a| all.contains(a)).is_some()))
    }

    fn outputs_of_exclude_pk(&self, pk: &PublicKey) -> RgResult<impl Iterator<Item=&Output>> {
        let all = pk.to_all_addresses()?;
        Ok(self.outputs.iter()
            .filter(move |o| o.address.as_ref().filter(|a| !all.contains(a)).is_some()))
    }

    fn output_rdg_amount_of_pk(&self, pk: &PublicKey) -> RgResult<CurrencyAmount> {
        Ok(self.outputs_of_pk(pk)?
            .filter_map(|a| a.opt_amount_typed())
            .filter(|a| a.currency_or() == SupportedCurrency::Redgold)
            .sum::<CurrencyAmount>())
    }

    fn output_rdg_amount_of_exclude_pk(&self, pk: &PublicKey) -> RgResult<CurrencyAmount> {
        Ok(self.outputs_of_exclude_pk(pk)?
            .filter_map(|a| a.opt_amount_typed())
            .filter(|a| a.currency_or() == SupportedCurrency::Redgold)
            .sum::<CurrencyAmount>())
    }

    fn first_input_address_to_btc_address(&self, network: &NetworkEnvironment) -> Option<String> {
        self.inputs.iter()
            .flat_map(|i| i.proof.iter().flat_map(|p| p.public_key.as_ref()))
            .next()
            .and_then(|public_other| { public_other.to_bitcoin_address(&network).ok() })
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