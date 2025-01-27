use bdk::database::BatchDatabase;
use redgold_schema::{error_info, structs, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{CurrencyAmount, Proof};
use crate::btc::btc_wallet::SingleKeyBitcoinWallet;
use crate::btc::threshold_multiparty::RawTransaction;
use crate::KeyPair;
use crate::proof_support::ProofSupport;

impl<D: BatchDatabase> SingleKeyBitcoinWallet<D> {

    // Same as below

    // Used for rendering json for gui
    pub fn prepare_single(&mut self, dest: String, amount: f64) -> RgResult<String> {
        let amount = (amount * (1e8f64)) as u64;
        self.create_transaction_output_batch(vec![(dest, amount)])?;
        self.render_json()
    }

    pub fn render_json(&self) -> RgResult<String> {
        RawTransaction {
            psbt: self.psbt.clone(),
            transaction_details: self.transaction_details.clone(),
        }.json()
    }

    pub fn prepare_single_sign(&mut self, dest: String, amount: f64, pkey_hex: String) -> RgResult<String> {
        self.prepare_single(dest, amount)?;
        self.local_sign_single(pkey_hex)
    }

    pub fn prepare_single_sign_and_broadcast(&mut self, dest: String, amount: f64, pkey_hex: String) -> RgResult<String> {
        self.prepare_single(dest, amount)?;
        self.local_sign_single(pkey_hex)?;
        self.broadcast_tx()?;
        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid.to_string();
        Ok(txid)
    }

    pub fn local_sign_single(&mut self, pkey_hex: String) -> RgResult<String> {
        let kp = KeyPair::from_private_hex(pkey_hex)?;
        let signables = self.signable_hashes()?;
        for (i, (hash, sighashtype)) in signables.iter().enumerate() {
            // println!("signable {}: {}", i, hex::encode(hash));
            let prf = Proof::from_keypair(hash, kp);
            self.affix_input_signature(i, &prf, sighashtype);
        }
        let finalized = self.sign()?;
        if !finalized {
            return Err(error_info("Not finalized"));
        }
        self.render_json()
    }

    pub fn send_local(&mut self, dest: String, amount: u64, pkey_hex: String) -> RgResult<String> {
        self.create_transaction_output_batch(vec![(dest, amount)])?;
        let kp = KeyPair::from_private_hex(pkey_hex)?;
        // let d = w.transaction_details.clone().expect("d");
        // println!("txid: {:?}", d.txid);
        let signables = self.signable_hashes()?;
        // println!("num signable hashes: {:?}", signables.len());
        for (i, (hash, sighashtype)) in signables.iter().enumerate() {
            // println!("signable {}: {}", i, hex::encode(hash));
            let prf = Proof::from_keypair(hash, kp);
            self.affix_input_signature(i, &prf, sighashtype);
        }
        let finalized = self.sign()?;
        if !finalized {
            return Err(error_info("Not finalized"));
        }
        // println!("finalized: {:?}", finalized);

        self.broadcast_tx()?;

        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid.to_string();
        // let txid = w.broadcast_tx().expect("txid");
        // println!("txid: {:?}", txid);
        Ok(txid)
    }

    pub fn send(&mut self, destination: &structs::Address, amount: &CurrencyAmount, pkey_hex: String, broadcast: bool) -> RgResult<String> {
        self.create_transaction_output_batch(vec![(destination.render_string()?, amount.amount as u64)])?;
        let kp = KeyPair::from_private_hex(pkey_hex)?;
        let signables = self.signable_hashes()?;
        for (i, (hash, sighashtype)) in signables.iter().enumerate() {
            // println!("signable {}: {}", i, hex::encode(hash));
            let prf = Proof::from_keypair(hash, kp);
            self.affix_input_signature(i, &prf, sighashtype);
        }
        let finalized = self.sign()?;
        if !finalized {
            return Err(error_info("Not finalized"));
        }

        if broadcast {
            self.broadcast_tx()?;
        }
        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid.to_string();
        // let txid = w.broadcast_tx().expect("txid");
        // println!("txid: {:?}", txid);
        Ok(txid)
    }
}