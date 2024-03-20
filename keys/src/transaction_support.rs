use std::collections::HashSet;
use redgold_schema::{EasyJson, error_code, error_info, error_message, ProtoSerde, RgResult, SafeOption, structs, WithMetadataHashable};
use redgold_schema::constants::{MAX_INPUTS_OUTPUTS};
use redgold_schema::structs::{Address, DebugSerChange, DebugSerChange2, ErrorInfo, Hash, Input, NetworkEnvironment, Proof, TimeSponsor, Transaction, TransactionOptions, UtxoEntry, UtxoId};
use redgold_schema::transaction::MAX_TRANSACTION_MESSAGE_SIZE;
use crate::KeyPair;
use crate::proof_support::ProofSupport;




use redgold_schema::util::current_time_millis;
use crate::address_external::ToBitcoinAddress;

pub trait TransactionSupport {
    fn time_sponsor(&mut self, key_pair: KeyPair) -> RgResult<Transaction>;
    fn sign(&mut self, key_pair: &KeyPair) -> Result<Transaction, ErrorInfo>;
    // TODO: Move all of this to TransactionBuilder
    fn verify_utxo_entry_proof(&self, utxo_entry: &UtxoEntry) -> Result<(), ErrorInfo>;
    fn input_bitcoin_address(&self, network: &NetworkEnvironment, other_address: &String) -> bool;
    fn output_swap_amount_of_multi(&self, pk_address: &structs::PublicKey, network_environment: &NetworkEnvironment) -> RgResult<i64>;
    fn output_amount_of_multi(&self, pk_address: &structs::PublicKey, network_environment: &NetworkEnvironment) -> RgResult<i64>;
    fn has_swap_to_multi(&self, pk_address: &structs::PublicKey, network_environment: &NetworkEnvironment) -> bool;
    fn first_input_address_to_btc_address(&self, network: &NetworkEnvironment) -> Option<String>;
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

    fn input_bitcoin_address(&self, network: &NetworkEnvironment, other_address: &String) -> bool {
        self.inputs.iter()
            .flat_map(|i| -> &Vec<Proof> { i.proof.as_ref()})
            .filter_map(|p: &structs::Proof| p.public_key.as_ref())
            .filter_map(|pk| pk.to_bitcoin_address(network).ok())
            .filter(|a| a == other_address)
            .count() > 0
    }

    fn output_swap_amount_of_multi(&self, pk_address: &structs::PublicKey, network_environment: &NetworkEnvironment) -> RgResult<i64> {
        let btc_address = pk_address.to_bitcoin_address(network_environment)?;
        let address = pk_address.address()?;
        let amt = self.outputs
            .iter()
            .filter_map(|o| {
                if o.is_swap() {
                    o.address.as_ref()
                        .filter(|&a| a == &address || a.render_string().ok().as_ref() == Some(&btc_address))
                        .and_then(|_| o.opt_amount())
                } else {
                    None
                }
            }).sum::<i64>();
        Ok(amt)
    }
    fn output_amount_of_multi(&self, pk_address: &structs::PublicKey, network_environment: &NetworkEnvironment) -> RgResult<i64> {
        let btc_address = pk_address.to_bitcoin_address(network_environment)?;
        let address = pk_address.address()?;
        let amt = self.outputs
            .iter()
            .filter_map(|o| {
                    o.address.as_ref()
                        .filter(|&a| a == &address || a.render_string().ok().as_ref() == Some(&btc_address))
                        .and_then(|_| o.opt_amount())
            }).sum::<i64>();
        Ok(amt)
    }

    fn has_swap_to_multi(&self, pk_address: &structs::PublicKey, network_environment: &NetworkEnvironment) -> bool {
        self.output_swap_amount_of_multi(pk_address, network_environment).map(|b| b > 0).unwrap_or(false)
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