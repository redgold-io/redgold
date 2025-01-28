use crate::btc::btc_wallet::SingleKeyBitcoinWallet;
use crate::util::mnemonic_support::MnemonicSupport;
use crate::TestConstants;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::Address;
use bdk::database::BatchDatabase;
use bdk::{FeeRate, SignOptions};
use itertools::Itertools;
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment};
use redgold_schema::{structs, ErrorInfoContext, RgResult};
use std::path::PathBuf;
use std::str::FromStr;


impl<D: BatchDatabase> SingleKeyBitcoinWallet<D> {
    pub fn get_descriptor_address(&self) -> RgResult<String> {
        // This gets the first address from the wallet's descriptor, which works for both single-key and multisig
        let address = self.wallet.get_address(bdk::wallet::AddressIndex::Peek(0))
            .error_info("Failed to get wallet address")?;
        Ok(address.to_string())
    }

    pub fn get_address_from_descriptor(&self) -> RgResult<structs::Address> {

        let addr = Default::default();
        Ok(addr)
    }

    pub fn create_multisig_transaction(
        &mut self,
        destination: &structs::Address,
        amount: &CurrencyAmount,
    ) -> Result<PartiallySignedTransaction, ErrorInfo> {
        // Sync wallet first to ensure UTXOs are up to date
        self.sync()?;

        let destination = destination.render_string()?;

        let destination_address = Address::from_str(&destination)
            .error_info("Invalid destination address")?;

        let mut builder = self.wallet.build_tx();

        builder
            .add_recipient(destination_address.script_pubkey(), amount.amount as u64)
            .enable_rbf()
            .fee_rate(FeeRate::from_sat_per_vb(self.sat_per_vbyte));

        let (psbt, details) = builder
            .finish()
            .error_info("Failed to build transaction")?;

        // Store transaction details
        self.transaction_details = Some(details);
        self.psbt = Some(psbt.clone());

        Ok(psbt)
    }

    pub fn sign_multisig_psbt(&mut self, mut psbt: PartiallySignedTransaction) -> Result<PartiallySignedTransaction, ErrorInfo> {
        self.wallet.sign(&mut psbt, SignOptions::default())
            .error_info("Failed to sign PSBT")?;

        Ok(psbt)
    }

    // Helper function to combine signatures from multiple wallets
    pub fn combine_psbts(
        &self,
        original_psbt: PartiallySignedTransaction,
        signed_psbts: Vec<PartiallySignedTransaction>
    ) -> Result<PartiallySignedTransaction, ErrorInfo> {
        let mut combined = original_psbt;

        for signed in signed_psbts {
            combined.combine(signed)
                .error_info("Failed to combine PSBTs")?;
        }

        Ok(combined)
    }

    // Check if PSBT is ready to be finalized
    pub fn is_psbt_finalized(&self, psbt: &PartiallySignedTransaction) -> bool {
        psbt.inputs.iter().all(|input| input.final_script_sig.is_some() || input.final_script_witness.is_some())
    }

    pub fn multisig_descriptor_create(pk_hex: String, peers: Vec<structs::PublicKey>, threshold: i64) -> RgResult<String> {
        let mut keys = vec![];
        keys.push(pk_hex);
        for pk in peers.iter() {
            keys.push(pk.to_hex_direct_ecdsa()?);
        }
        keys.sort();

        println!("keys: {:?}", keys);

        // Create descriptor with proper format for BDK
        let keys_str = keys.join(",");
        let descriptor_str = format!("wsh(multi({},{}))", threshold, keys_str);

        println!("descriptor_str: {:?}", descriptor_str);

        // Use BDK's descriptor parser to get proper checksum
        let descriptor = bdk::descriptor::Descriptor::<bdk::bitcoin::PublicKey>::from_str(&descriptor_str)
            .error_info("Failed to parse descriptor")?
            .to_string();

        Ok(descriptor)
    }

}

#[ignore]
#[tokio::test]
async fn balance_test_mn() {
    let ci = TestConstants::test_words_pass().unwrap();
    let kp = ci.keypair_at("m/84'/0'/0'/0/0").expect("");
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();
    let path = TestConstants::dev_ci_kp_path();
    let pkh = ci.private_at(path.clone()).unwrap();
    let pkh1 = ci1.private_at(path.clone()).unwrap();
    let pkh2 = ci2.private_at(path.clone()).unwrap();
    let pub0 = ci.public_at(path.clone()).unwrap();
    let pub1 = ci1.public_at(path.clone()).unwrap();
    let pub2 = ci2.public_at(path.clone()).unwrap();

    let pubkeys = vec![pub0.clone(), pub1.clone(), pub2.clone()];

    let pb = PathBuf::from("testdb");
    let pb1 = PathBuf::from("testdb1");
    let pb2 = PathBuf::from("testdb2");

    std::fs::remove_dir_all(&pb).ok();
    std::fs::remove_dir_all(&pb1).ok();
    std::fs::remove_dir_all(&pb2).ok();

    let wm = SingleKeyBitcoinWallet::new_wallet_db_backed(
        pub0.clone(),
        NetworkEnvironment::Dev,
        true,
        pb,
        None,
        // Pass None for the change descriptor by using tr()
        Some(pkh),
        Some(vec![pub1.clone(), pub2.clone()]),
        Some(2),
    ).expect("Failed to create wallet");

    let wm1 = SingleKeyBitcoinWallet::new_wallet_db_backed(
        pub1.clone(),
        NetworkEnvironment::Dev,
        true,
        pb1,
        None,
        // Pass None for the change descriptor by using tr()
        Some(pkh1),
        Some(vec![pub2.clone(), pub0.clone()]),
        Some(2),
    ).expect("Failed to create wallet");

    let balance = wm.get_wallet_balance().expect("Failed to get balance");
    println!("balance: {:?}", balance);
    let addr = wm.get_descriptor_address().expect("Failed to get descriptor address");
    println!("addr: {:?}", addr);


    let mut w2 = SingleKeyBitcoinWallet::new_wallet(
        pub0.clone(), NetworkEnvironment::Dev, true
    ).expect("Failed to create wallet");

    let balance2 = w2.get_wallet_balance().expect("Failed to get balance");
    println!("balance2: {:?}", balance2);
    let addr2 = w2.get_descriptor_address().expect("Failed to get descriptor address");
    println!("addr2: {:?}", addr2);

    assert_eq!(wm.get_descriptor_address().unwrap(), wm1.get_descriptor_address().unwrap())

    // let res = w2.send_local(addr, 8000, pkh).expect("Failed to send");
    // println!("res: {:?}", res);

    // w.create_multisig_transaction()

}
