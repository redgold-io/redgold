use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::{BatchDatabase, MemoryDatabase};
use bdk::electrum_client::Client;
use bdk::Wallet;
use redgold_schema::{structs, ErrorInfoContext, RgResult};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, PublicKey};
use crate::address_external::ToBitcoinAddress;
use crate::btc::btc_wallet::{network_to_backends, SingleKeyBitcoinWallet};
use crate::btc::threshold_multiparty::MultipartySigner;
use crate::TestConstants;
use crate::util::mnemonic_support::MnemonicSupport;


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

    // Convert public keys to hex format and add the required "02" or "03" prefix
    let pubkey_hexes: Vec<String> = pubkeys.iter()
        .map(|pk| {
            let hex = pk.to_hex_direct_ecdsa().unwrap();
            hex
        })
        .collect();

    let thresh = 2;
    
    // Create descriptor with proper format for BDK
    let keys_str = pubkey_hexes.join(",");
    let descriptor_str = format!("wsh(multi({},{}))", thresh, keys_str);
    
    println!("Descriptor string: {}", descriptor_str);
    
    // Use BDK's descriptor parser to get proper checksum
    let descriptor = bdk::descriptor::Descriptor::<bdk::bitcoin::PublicKey>::from_str(&descriptor_str)
        .expect("Failed to parse descriptor")
        .to_string();

    println!("Final descriptor with checksum: {}", descriptor);
    let pb = PathBuf::from("testdb");

    std::fs::remove_dir_all(&pb).unwrap();

    let w = SingleKeyBitcoinWallet::new_wallet_db_backed(
        kp.public_key(),
        NetworkEnvironment::Dev,
        true,
        pb,
        None,
        // Pass None for the change descriptor by using tr()
        Some(descriptor)
    ).expect("Failed to create wallet");

    let balance = w.get_wallet_balance().expect("Failed to get balance");
    println!("balance: {:?}", balance);
    let addr = w.get_descriptor_address().expect("Failed to get descriptor address");
    println!("addr: {:?}", addr);


    let mut w2 = SingleKeyBitcoinWallet::new_wallet(
        pub0.clone(), NetworkEnvironment::Dev, true
    ).expect("Failed to create wallet");

    let balance2 = w2.get_wallet_balance().expect("Failed to get balance");
    println!("balance2: {:?}", balance2);
    let addr2 = w2.get_descriptor_address().expect("Failed to get descriptor address");
    println!("addr2: {:?}", addr2);
    let res = w2.send_local(addr, 8000, pkh).expect("Failed to send");
    println!("res: {:?}", res);

}
