// use bdk::bitcoin::hashes::sha256d;
// use bdk::bitcoin::network::constants::Network;
// use bdk::bitcoin::{OutPoint, Script, SigHashType, Transaction, TxIn, TxOut};
// use bdk::bitcoin_wallet::account::{
//     Account, AccountAddressType, MasterAccount, MasterKeyEntropy, Unlocker,
// };
// use bdk::bitcoin_wallet::mnemonic::Mnemonic;
// use std::collections::HashMap;
//
// #[test]
// fn test_example() {
//     const PASSPHRASE: &str = "correct horse battery staple";
//
//     // create a new random master account. This holds the root BIP32 key
//     // PASSPHRASE is used to encrypt the seed in memory and in storage
//     let mut master =
//         MasterAccount::new(MasterKeyEntropy::Sufficient, Network::Bitcoin, PASSPHRASE).unwrap();
//
//     // or re-create a master from a known mnemonic
//     let words = "announce damage viable ticket engage curious yellow ten clock finish burden orient faculty rigid smile host offer affair suffer slogan mercy another switch park";
//     let mnemonic = Mnemonic::from_str(words).unwrap();
//     // PASSPHRASE is used to encrypt the seed in memory and in storage
//     // last argument is option password for plausible deniability
//     let mut master =
//         MasterAccount::from_mnemonic(&mnemonic, 0, Network::Bitcoin, PASSPHRASE, None).unwrap();
//     // The master accounts only store public keys
//     // Private keys are created on-demand from encrypted seed with an Unlocker and forgotten as soon as possible
//
//     // create an unlocker that is able to decrypt the encrypted mnemonic and then calculate private keys
//     let mut unlocker = Unlocker::new_for_master(&master, PASSPHRASE).unwrap();
//
//     // The unlocker is needed to create accounts within the master account as
//     // key derivation follows BIP 44, which requires private key derivation
//
//     // create a P2PKH (pay-to-public-key-hash) (legacy) account.
//     // account number 0, sub-account 0 (which usually means receiver) BIP32 look-ahead 10
//     let account = Account::new(&mut unlocker, AccountAddressType::P2PKH, 0, 0, 10).unwrap();
//     master.add_account(account);
//
//     // create a P2SHWPKH (pay-to-script-hash-witness-public-key-hash) (transitional single key segwit) account.
//     // account number 0, sub-account 1 (which usually means change) BIP32 look-ahead 10
//     let account = Account::new(&mut unlocker, AccountAddressType::P2SHWPKH, 0, 1, 10).unwrap();
//     master.add_account(account);
//
//     // create a P2WPKH (pay-to-witness-public-key-hash) (native single key segwit) account.
//     // account number 1, sub-account 0 (which usually means receiver) BIP32 look-ahead 10
//     let account = Account::new(&mut unlocker, AccountAddressType::P2WPKH, 1, 0, 10).unwrap();
//     master.add_account(account);
//     // account number 1, sub-account 1 (which usually means change) BIP32 look-ahead 10
//     let account = Account::new(&mut unlocker, AccountAddressType::P2WPKH, 1, 1, 10).unwrap();
//     master.add_account(account);
//
//     // get next legacy receiver address
//     let source = master
//         .get_mut((0, 0))
//         .unwrap()
//         .next_key()
//         .unwrap()
//         .address
//         .clone();
//     // pay to some native segwit address
//     let target = master
//         .get_mut((1, 0))
//         .unwrap()
//         .next_key()
//         .unwrap()
//         .address
//         .clone();
//     // change to some transitional address
//     let change = master
//         .get_mut((0, 1))
//         .unwrap()
//         .next_key()
//         .unwrap()
//         .address
//         .clone();
//
//     // a dummy transaction to send to source
//     let input_transaction = Transaction {
//         input: vec![TxIn {
//             previous_output: OutPoint {
//                 txid: sha256d::Hash::default(),
//                 vout: 0,
//             },
//             sequence: RBF,
//             witness: Vec::new(),
//             script_sig: Script::new(),
//         }],
//         output: vec![TxOut {
//             script_pubkey: source.script_pubkey(),
//             value: 5000000000,
//         }],
//         lock_time: 0,
//         version: 2,
//     };
//
//     let txid = input_transaction.txid();
//
//     const RBF: u32 = 0xffffffff - 2;
//
//     // a dummy transaction that spends source
//     let mut spending_transaction = Transaction {
//         input: vec![TxIn {
//             previous_output: OutPoint { txid, vout: 0 },
//             sequence: RBF,
//             witness: Vec::new(),
//             script_sig: Script::new(),
//         }],
//         output: vec![
//             TxOut {
//                 script_pubkey: target.script_pubkey(),
//                 value: 4000000000,
//             },
//             TxOut {
//                 script_pubkey: change.script_pubkey(),
//                 value: 999999000,
//             },
//         ],
//         lock_time: 0,
//         version: 2,
//     };
//
//     // helper to find previous outputs
//     let mut spent = HashMap::new();
//     spent.insert(txid, input_transaction.clone());
//
//     // sign the spend
//     master
//         .sign(
//             &mut spending_transaction,
//             SigHashType::All,
//             &(|_| Some(input_transaction.output[0].clone())),
//             &mut unlocker,
//         )
//         .expect("can not sign");
//
//     // verify the spend with the bitcoinconsensus library
//     spending_transaction
//         .verify(|point| {
//             spent
//                 .get(&point.txid)
//                 .and_then(|t| t.output.get(point.vout as usize).cloned())
//         })
//         .expect("Bitcoin Core would not like this")
// }
