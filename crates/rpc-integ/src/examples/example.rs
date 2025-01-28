use ethers::{core::types::TransactionRequest,
             middleware::SignerMiddleware, providers::{Http, Middleware, Provider}, signers::{LocalWallet, Signer}};


use redgold_schema::keys::words_pass::WordsPass;

use crate::eth::eth_wallet::EthWalletWrapper;
use crate::eth::historical_client::EthHistoricalClient;
use alloy_chains::Chain;
use ethers::prelude::U256;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::utils::Anvil;
use foundry_block_explorers::Client;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::{KeyPair, TestConstants};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment};
use redgold_schema::util::lang_util::AnyPrinter;

//  Has faucet bitcoin test funds
pub fn dev_ci_kp() -> Option<(String, KeyPair)> {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        // This is wrong for ethereum, but we just need the secret key to match other
        // faucet funds for 'same' key.
        let path = "m/84'/0'/0'/0/0";
        let privk = w.private_at(path.to_string()).expect("private key");
        let keypair = w.keypair_at(path.to_string()).expect("private key");
        Some((privk, keypair))
    } else {
        None
    }
}

fn eth_addr() -> String {
    "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".to_string()
}


async fn foo() -> Result<(), Box<dyn std::error::Error>> {

    let key = std::env::var("ETHERSCAN_API_KEY").expect("api key");
    let client = Client::new(Chain::sepolia(), key).expect("works");
    // Or using environment variables
    // let client = Client::new_from_env(Chain::mainnet())?;

    let address = "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".parse().expect("valid address");
    let metadata = client.get_ether_balance_single(&address, None).await.expect("works");
    // assert_eq!(metadata.items[0].contract_name, "DAO");
    println!("balance: {}", metadata.balance);

    let _txs = client.get_transactions(&address, None).await.expect("works");

    let environment = NetworkEnvironment::Dev;
    let h = EthHistoricalClient::new(&environment).expect("works").expect("works");

    let string_addr = "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".to_string();
    let txs = h.get_all_tx(&string_addr, None).await.expect("works");

    println!("txs: {}", txs.json_or());

    let tx_head = txs.get(0).expect("tx");
    let _from = tx_head.other_address.clone();

    let (dev_secret, _dev_kp) = dev_ci_kp().expect("works");

    let _eth = EthWalletWrapper::new(&dev_secret, &environment).expect("works");

    let fee = "0.000108594791676".to_string();
    let fee_value = EthHistoricalClient::translate_value(&fee.to_string()).expect("works") as u64;
    let _amount = fee_value * 2;
    // eth.create_transaction(&from, amount).await.expect("works");

    Ok(())
}

// 0xA729F9430fc31Cda6173A0e81B55bBC92426f759
#[ignore]
#[tokio::test]
async fn main() {
    foo().await.expect("works");

    let _api_key = std::env::var("ETHERSCAN_API_KEY").expect("");

    let testc = TestConstants::new();
    // let _test_skhex = testc.key_pair().secret_key.to_hex();

    let (dev_secret, dev_kp) = dev_ci_kp().expect("works");

    let bytes = hex::decode(dev_secret).expect("wtf");

    let _eth = dev_kp.public_key().to_ethereum_address().expect("works").print();

    let w = LocalWallet::from_bytes(&bytes).expect("works");
    println!("Wallet btc: {}", w.address().to_string());

    let anvil = Anvil::new().spawn();

    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let wallet2: LocalWallet = anvil.keys()[1].clone().into();

    // connect to the network
    let provider = Provider::<Http>::try_from(anvil.endpoint()).expect("works");


    // connect the wallet to the provider
    let wallet1 = wallet.with_chain_id(anvil.chain_id());
    let client = SignerMiddleware::new(
        provider, wallet1.clone()
    );


    let addr = wallet1.address();
    let hexs = hex::encode(addr.0);
    println!("Wallet 1 address: {}", hexs);

    let balance = client.get_balance(wallet1.address(), None).await.expect("works");

    println!("Wallet 1 balance: {}", balance);

    // craft the transaction
    let tx = TransactionRequest::new().to(wallet2.address()).value(10000);

    // send it!
    let pending_tx = client.send_transaction(tx, None).await.expect("works");

    // get the mined tx
    let receipt = pending_tx.await.expect("mined").expect("no error");
    let tx = client.get_transaction(receipt.transaction_hash).await.expect("works");

    println!("Sent tx: {}\n", serde_json::to_string(&tx).expect("works"));
    println!("Tx receipt: {}", serde_json::to_string(&receipt).expect("works"));


}
// 0xA729F9430fc31Cda6173A0e81B55bBC92426f759


#[ignore]
#[tokio::test]
async fn debug_u256() {

    let tc = TestConstants::new();
    let (dev_secret, _dev_kp) = dev_ci_kp().expect("works");

    let eth = EthWalletWrapper::new(&dev_secret, &NetworkEnvironment::Dev).expect("works");
    let eth_addr = _dev_kp.public_key().to_ethereum_address_typed().expect("works");
    let eth_addr2 = tc.key_pair().public_key().to_ethereum_address_typed().expect("works");
    let tx = eth.create_transaction_typed(&eth_addr, &eth_addr2, CurrencyAmount::from_eth_fractional(0.001), None).await.expect("works");
    let val = tx.value().expect("works");
    let str = val.to_string();
    println!("tx: {}", str);
    let u25 = U256::from_dec_str(&str).expect("works");
    assert_eq!(val, &u25);

    println!("tx: {}", tx.json_or());

    let tx_js = tx.json_or();

    println!("tx: {:?}", tx);
    let tx2 = tx_js.json_from::<TypedTransaction>().expect("works");
    println!("tx2: {:?}", tx2);

    let signing = EthWalletWrapper::signing_data(&tx).expect("works");
    //
    // EthWalletWrapper::validate_eth_fulfillment(
    //     vec![(eth_addr2.clone(), CurrencyAmount::from_eth_fractional(0.001))],
    //     &tx_js,
    //     &signing,
    //     &NetworkEnvironment::Dev,
    // ).expect("works");

}
