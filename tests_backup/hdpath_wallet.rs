// use bitcoin::{
//     network::constants::Network,
//     secp256k1::Secp256k1,
//     util::bip32::{DerivationPath, ExtendedPrivKey},
// };
// use bitcoin_wallet::account::MasterAccount;
// use bitcoin_wallet::mnemonic::Mnemonic;
// use hdpath::StandardHDPath;
// use std::str::FromStr;
//
// #[test]
// fn replicate_example() {
//     let hd_path = StandardHDPath::from_str("m/44'/0'/0'/0/0").unwrap();
//     //prints "m/44'/0'/0'/0/0"
//     //     println!("{:?}", hd_path);
//     //
//     // //prints "0", which is account id
//     //     println!("{:?}", hd_path.account());
//     //
//     // //prints: "purpose: Pubkey, coin: 0, account: 0, change: 0, index: 0"
//     //     println!("purpose: {:?}, coin: {}, account: {}, change: {}, index: {}",
//     //              hd_path.purpose(),
//     //              hd_path.coin_type(),
//     //              hd_path.account(),
//     //              hd_path.change(),
//     //              hd_path.index()
//     //     );
//
//     const PASSPHRASE: &str = "correct horse battery staple";
//
//     // or re-create a master from a known mnemonic
//     let words = "announce damage viable ticket engage curious yellow ten clock finish burden orient faculty rigid smile host offer affair suffer slogan mercy another switch park";
//     // let words = "fork popular abandon state expire always dash girl basic decrease aisle manual fatal border hunt slide tuna regret correct legend congress result feel repair";
//     let mnemonic = Mnemonic::from_str(words).unwrap();
//
//     let mnem_seed = mnemonic.to_seed(None);
//
//     //let seed = master.seed(Network::Bitcoin, PASSPHRASE).unwrap();
//
//     let epk = redgold::util::wallet::get_pk(&mnem_seed.0, &hd_path);
//
//     println!("epk: {}", epk.to_string());
//
//     let hd_path_eth = StandardHDPath::from_str("m/44'/60'/0'/0/0").unwrap();
//
//     let epk_eth = redgold::util::wallet::get_pk(&mnem_seed.0, &hd_path_eth);
//
//     // println!("epk_eth: {}", epk_eth.to_string());
//
//     // This compares properly to python hdwallet
// }
