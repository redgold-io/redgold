use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::eth::safe_multisig::SafeMultisig;
use redgold_keys::TestConstants;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, SupportedCurrency};
use redgold_schema::structs::SupportedCurrency::Ethereum;

pub fn dev_ci_kp_path() -> String {
    "m/84'/0'/0'/0/0".to_string()
}

// #[ignore]
#[tokio::test]
pub async fn test_safe_multisig() {
    let ci = TestConstants::test_words_pass().unwrap();
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();
    let path = dev_ci_kp_path();
    let pkh = ci.private_at(path.clone()).unwrap();
    let pkh1 = ci1.private_at(path.clone()).unwrap();
    let pkh2 = ci2.private_at(path.clone()).unwrap();
    let addr = ci.public_at(path.clone()).unwrap().to_ethereum_address_typed().unwrap();
    let addr1 = ci1.public_at(path.clone()).unwrap().to_ethereum_address_typed().unwrap();
    let addr2 = ci2.public_at(path.clone()).unwrap().to_ethereum_address_typed().unwrap();

    let safe = SafeMultisig::new(NetworkEnvironment::Dev, addr.clone(), pkh.clone());
    let safe1 = SafeMultisig::new(NetworkEnvironment::Dev, addr1.clone(), pkh1.clone());
    let safe2 = SafeMultisig::new(NetworkEnvironment::Dev, addr2.clone(), pkh2.clone());

    let addrs = vec![addr.clone(), addr1.clone(), addr2.clone()];
    // let res = safe.create_safe(2, addrs).await.unwrap();

    let safe_contract_addr = Address::from_eth_external("0x449F629b6bf816db771b69388E5b02b30ED86ACe");

    // println!("pk {}", pkh);
    // println!("pk1 {}", pkh1);
    // println!("pk2 {}", pkh2);

    let eth_amount = CurrencyAmount::from_fractional_cur(0.001, SupportedCurrency::Ethereum).unwrap();
    // println!("ETH AMOUNT {:?}", eth_amount.string_amount);
    let w = EthWalletWrapper::new(&pkh, &NetworkEnvironment::Dev).unwrap();
    let to = addr1.clone();
    // let (tx_data, signature) = w.create_and_sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();
    // println!("TX DATA {}", hex::encode(tx_data.clone()));
    // println!("SIGNATURE {}", hex::encode(signature.clone()));
    // let data = EthWalletWrapper::decode_safe_tx_data(tx_data.as_ref()).unwrap();

    let w1 = EthWalletWrapper::new(&pkh1, &NetworkEnvironment::Dev).unwrap();
    let w2 = EthWalletWrapper::new(&pkh2, &NetworkEnvironment::Dev).unwrap();
    let res = w.sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();
    let res1 = w1.sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();
    let res2 = w2.sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();

    let res = w.execute_safe_transaction(&safe_contract_addr, &to, &eth_amount, vec![res, res1, res2]).await.unwrap();






    // println!("ETH ADDR {}", addr.render_string().unwrap());
    // println!("BTC ADDR main {}", ci.public_at(path.clone()).unwrap()
    //     .to_bitcoin_address_typed(&NetworkEnvironment::Main).unwrap().render_string().unwrap());
    // println!("BTC ADDR {}", ci.public_at(path.clone()).unwrap()
    //     .to_bitcoin_address_typed(&NetworkEnvironment::Dev).unwrap().render_string().unwrap());
    // println!("RDG ADDR {}", ci.public_at(path.clone()).unwrap()
    //     .address().unwrap().render_string().unwrap());

    // let eth = EthHistoricalClient::new(&NetworkEnvironment::Dev)
    //     .unwrap().unwrap();
    //
    // let to = safe.factory_address();
    // let from = addr.clone();
    //

    // let contracts = eth.get_all_deployed_contracts(&from, &to, None).await.unwrap();
    //
    // println!("Contracts {}", contracts.len());
    // for c in contracts {
    //     println!("Contract {}", c.render_string().unwrap());
    // }
    //
    // let all = eth.get_all_raw_tx(&from, None).await.unwrap();
    // println!("All tx {}", all.len());
    // for tx in all {
    //     println!("Tx {}", tx.json_or());
    // }

}