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

#[ignore]
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

    let eth_amount = CurrencyAmount::from_fractional_cur(0.001, SupportedCurrency::Ethereum).unwrap();
    let w = EthWalletWrapper::new(&pkh, &NetworkEnvironment::Dev).unwrap();
    let to = addr1.clone();
    println!("To {}", to.render_string().unwrap());

    let w1 = EthWalletWrapper::new(&pkh1, &NetworkEnvironment::Dev).unwrap();
    let w2 = EthWalletWrapper::new(&pkh2, &NetworkEnvironment::Dev).unwrap();
    let (tx_hash, res) = w.sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();
    let (t1, res1) = w1.sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();
    // let (t2, res2) = w2.sign_safe_tx(&safe_contract_addr, &to, &eth_amount).await.unwrap();
    assert_eq!(tx_hash, t1);
    // assert_eq!(tx_hash, t2);
    // EthWalletWrapper::combine_signatures(tx_hash, vec![res, res1, res2]).unwrap();

    // println!("Actual addresses: {}", addrs.iter().map(|x| x.render_string().unwrap()).collect::<Vec<String>>().join(", "));
    //
    //
    let res = w.execute_safe_transaction(
        &safe_contract_addr, &to, &eth_amount, vec![res, res1], tx_hash).await.unwrap();

}