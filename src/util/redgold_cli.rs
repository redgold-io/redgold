// // extern crate redgold;
// //
// // use bitcoin_wallet::mnemonic::Mnemonic;
// // use hex;
// // use crate::mnemonic_builder;
// // use crate::util;
// // use crate::util::wallet::Wallet;
// //
// // //https://stackoverflow.com/questions/27650312/show-u8-slice-in-hex-representation
// // // https://docs.docker.com/docker-hub/
// //
// // use std::fmt;
// // use std::fs::File;
// // use std::io::Write;
// //
// // use bitcoin::hashes::hex::ToHex;
// // use bitcoin::{
// //     network::constants::Network,
// //     secp256k1::Secp256k1,
// //     util::bip32::{DerivationPath, ExtendedPrivKey},
// //     Address,
// // };
// // use bitcoin_wallet::account::MasterAccount;
// // use hdpath::StandardHDPath;
// // use std::str::FromStr;
// //
// // // For some reason hexSlice below isn't working ??
// //
// // #[derive(Debug)]
// // struct HexSlice<'a>(&'a [u8]);
// //
// // impl<'a> HexSlice<'a> {
// //     fn new<T>(data: &'a T) -> HexSlice<'a>
// //     where
// //         T: ?Sized + AsRef<[u8]> + 'a,
// //     {
// //         HexSlice(data.as_ref())
// //     }
// // }
// //
// // // You can choose to implement multiple traits, like Lower and UpperHex
// // impl fmt::Display for HexSlice<'_> {
// //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
// //         for byte in self.0 {
// //             // Decide if you want to pad the value or have spaces inbetween, etc.
// //             write!(f, "{:X} ", byte)?;
// //         }
// //         Ok(())
// //     }
// // }
// //
// // //
// // // fn main() {
// // //     // To get a `String`
// // //     let s = format!("{}", HexSlice::new("Hello"));
// // //
// // //     // Or print it directly
// // //     println!("{}", HexSlice::new("world"));
// // //
// // //     // Works with
// // //     HexSlice::new("Hello"); // string slices (&str)
// // //     HexSlice::new(b"Hello"); // byte slices (&[u8])
// // //     HexSlice::new(&"World".to_string()); // References to String
// // //     HexSlice::new(&vec![0x00, 0x01]); // References to Vec<u8>
// // // }
// //
// // // (Full example with detailed comments in examples/01d_quick_example.rs)
// // //
// // // This example demonstrates clap's full 'custom derive' style of creating arguments which is the
// // // simplest method of use, but sacrifices some flexibility.
// // use clap::{AppSettings, Clap};
// // use crate::util::KeyPair;
// //
// // /// This doc string acts as a help message when the user runs '--help'
// // /// as do all doc strings on fields
// // #[derive(Clap)]
// // #[clap(version = "0.1", author = "")]
// // #[clap(setting = AppSettings::ColoredHelp)]
// // struct Opts {
// //     /// Sets a custom config file. Could have been an Option<T> with no default too
// //     // #[clap(short, long, default_value = "default.conf")]
// //     // config: String,
// //     // /// Some input. Because this isn't an Option<T> it's required to be used
// //     // input: String,
// //     // /// A level of verbosity, and can be used multiple times
// //     // #[clap(short, long, parse(from_occurrences))]
// //     // verbose: i32,
// //     #[clap(subcommand)]
// //     subcmd: SubCommand,
// // }
// //
// // #[derive(Clap)]
// // enum SubCommand {
// //     #[clap(version = "1.3", author = "Redgold")]
// //     GenerateMnemonic(GenerateMnemonic),
// //     DumpPublic(DumpPublic),
// //     BitcoinAddress(BitcoinAddress),
// //     GeneratePeerId(GeneratePeerId),
// // }
// //
// // /// Generate a mnemonic from a password (minimum 128 bits of entropy required)
// // #[derive(Clap)]
// // struct GenerateMnemonic {
// //     /// Print debug info
// //     #[clap(short, long)]
// //     password: String,
// //     #[clap(short, long, default_value = "10000")]
// //     rounds: i32,
// // }
// //
// // /*
// // 1m keys
// // `1000 mnemonics
// // leaves 1000 other keys
// // 50 transport keys
// // 50 observation keys
// // 5 reward keys
// // 895 deposit keys.
// //
// // KeyTypes:
// // Transport
// // Observation
// // Deposit
// // Reward (probably not should not be on node, but necessary as an optional thing)
// // External Coin Keys?
// //
// //
// //  */
// // /// Generate a Peer ID for various key types
// // #[derive(Clap)]
// // struct GeneratePeerId {
// //     /// Generation password (cold)
// //     #[clap(short, long)]
// //     password: String,
// //     /// Round offset to begin from, use one that is
// //     #[clap(short, long, default_value = "10000")]
// //     rounds: i32,
// //     #[clap(short, long)]
// //     mnemonic_passphrase: Option<String>,
// //     #[clap(short, long, default_value = "1000")]
// //     num_mnemonics: i32,
// //     #[clap(short, long, default_value = "1000")]
// //     show_mnemonics: i32,
// //     #[clap(short, long, default_value = "50")]
// //     transport_keys: i32,
// //     #[clap(short, long, default_value = "50")]
// //     observation_keys: i32,
// //     #[clap(short, long, default_value = "895")]
// //     deposit_keys: i32,
// //     #[clap(short, long, default_value = "5")]
// //     reward_keys: i32,
// // }
// //
// // // wallet command structure?
// // // unlock wallet?
// // // subcommand actions?
// //
// // // https://www.kaggle.com/lsind18/full-moon-calendar-1900-2050
// // /// Debug Command for generating genesis transaction (which is really a reward)
// // #[derive(Clap)]
// // struct GenerateGenesis {
// //     /// Generation password (cold)
// //     #[clap(short, long)]
// //     password: String,
// //     /// Round offset to begin from, use one that is
// //     #[clap(short, long, default_value = "10000")]
// //     rounds: i32,
// //     #[clap(short, long)]
// //     mnemonic_passphrase: Option<String>,
// // }
// //
// // //
// // // /// Generate a mnemonic from a password (minimum 128 bits of entropy required)
// // // #[derive(Clap)]
// // // struct GeneratePeerIdMnemonics {
// // //     /// Print debug info
// // //     #[clap(short, long)]
// // //     password: String,
// // //     #[clap(short, long, default_value = "10000")]
// // //     rounds: i32,
// // //     #[clap(short, long)]
// // //     menmonic_passphrase: Option<String>,
// // //     #[clap(short, long, default_value = "20")]
// // //     num_mnemonics: i32,
// // //     #[clap(short, long, default_value = "20")]
// // //     accounts_per_mnemonic: i32,
// // //     #[clap(short, long, default_value = "20")]
// // //     changes_per_account: i32,
// // //     #[clap(short, long, default_value = "1000")]
// // //     keys_per_change: i32,
// // // }
// //
// // /// Dump a public key at a path in hex format
// // #[derive(Clap)]
// // struct BitcoinAddress {
// //     /// Print debug info
// //     #[clap(short, long)]
// //     mnemonic: String,
// //     #[clap(short, long)]
// //     passphrase: Option<String>,
// //     #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
// //     key_path: String,
// // }
// //
// // /// Dump a bitcoin address at a path in hex format
// // #[derive(Clap)]
// // struct DumpPublic {
// //     /// Print debug info
// //     #[clap(short, long)]
// //     mnemonic: String,
// //     #[clap(short, long)]
// //     passphrase: Option<String>,
// //     #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
// //     key_path: String,
// // }
// //
// // #[test]
// // fn convert_to_bitcoin_address() {
// //     let m = "cactus scare honey cycle whale asset motion dynamic pear wild brisk curious various evoke lonely loan dumb express antenna hood chaos feed square brass";
// //     let w = Wallet::from_mnemonic(m, None);
// //     let public_key = w.key_from_path_str("m/84'/0'/0'/0/0".to_string()).1;
// //     // println!("{}", public_key.to_hex());
// //     use bitcoin::util::key;
// //     let address = Address::p2wpkh(
// //         &key::PublicKey::from_str(&*public_key.to_string()).unwrap(),
// //         Network::Bitcoin,
// //     );
// //     println!("{}", address);
// // }
// //
// // pub fn generate_sub_key_hash(wallet: &Wallet, offset: usize, iteration: i32) -> [u8; 32] {
// //     let pair = wallet.keypair_from_path_str(
// //         format!("m/84'/1854715124'/0'/{:?}/{:?}", offset, iteration).to_string(),
// //     );
// //     util::dhash_vec(&pair.public_key.serialize().to_vec())
// // }
// //
// // fn main() {
// //     let opts: Opts = Opts::parse();
// //     //
// //     // // Gets a value for config if supplied by user, or defaults to "default.conf"
// //     // println!("Value for config: {}", opts.config);
// //     // println!("Using input file: {}", opts.input);
// //
// //     // Vary the output based on how many times the user used the "verbose" flag
// //     // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
// //     // match opts.verbose {
// //     //     0 => println!("No verbose info"),
// //     //     1 => println!("Some verbose info"),
// //     //     2 => println!("Tons of verbose info"),
// //     //     _ => println!("Don't be ridiculous"),
// //     // }
// //
// //     // You can handle information about subcommands by requesting their matches by name
// //     // (as below), requesting just the name used, or both at the same time
// //     match opts.subcmd {
// //         SubCommand::GenerateMnemonic(t) => {
// //             println!("{}, {:?}, {:?}", t.password, t.rounds, t.password.len());
// //             let m1 = mnemonic_builder::from_str_rounds(&*t.password, t.rounds as usize);
// //             println!("{}", m1.to_string())
// //         }
// //         SubCommand::DumpPublic(d) => {
// //             let w = Wallet::from_mnemonic(&*d.mnemonic, d.passphrase);
// //             println!("{}", w.key_from_path_str(d.key_path).1.to_hex());
// //         }
// //         //
// //         SubCommand::BitcoinAddress(d) => {
// //             let w = Wallet::from_mnemonic(&*d.mnemonic, d.passphrase);
// //             let public_key = w.key_from_path_str(d.key_path).1;
// //             use bitcoin::util::key;
// //             let address = Address::p2wpkh(
// //                 &key::PublicKey::from_str(&*public_key.to_string()).unwrap(),
// //                 Network::Bitcoin,
// //             );
// //             println!("{}", address);
// //         }
// //         SubCommand::GeneratePeerId(p) => {
// //             let original_mnemonic =
// //                 mnemonic_builder::from_str_rounds_preserve_hash(&*p.password, p.rounds as usize);
// //             println!(
// //                 "{}, {:?}, {:?}, {:?}",
// //                 p.password,
// //                 p.rounds,
// //                 p.password.len(),
// //                 hex::encode(original_mnemonic.checksum())
// //             );
// //             println!("Original mnemonic:");
// //             println!("{}", original_mnemonic.mnemonic.to_string());
// //
// //             let password = p.password + "Redgold_peer_id_salt";
// //             let m1 = mnemonic_builder::from_str_rounds_preserve_hash(&*password, p.rounds as usize);
// //             println!(
// //                 "{}, {:?}, {:?}, {:?}",
// //                 password,
// //                 p.rounds,
// //                 password.len(),
// //                 hex::encode(m1.checksum())
// //             );
// //             println!("Master mnemonic:");
// //             println!("{}", m1.mnemonic.to_string());
// //             let mut key_hashes: Vec<[u8; 32]> = vec![];
// //
// //             for m in 0..p.num_mnemonics {
// //                 let m_prime = m1.offset_derivation_path_salted_hash(&*m.to_string());
// //                 println!("Offset {:?} mnemonic:", m);
// //                 println!("{}", m_prime.mnemonic.to_string());
// //                 let wallet = Wallet::from_mnemonic(&*m_prime.mnemonic.to_string(), None);
// //
// //                 for i in 0..p.transport_keys {
// //                     key_hashes.push(generate_sub_key_hash(&wallet, 0, i));
// //                 }
// //
// //                 for i in 0..p.observation_keys {
// //                     key_hashes.push(generate_sub_key_hash(&wallet, 1, i));
// //                 }
// //
// //                 for i in 0..p.deposit_keys {
// //                     key_hashes.push(generate_sub_key_hash(&wallet, 2, i));
// //                 }
// //
// //                 for i in 0..p.reward_keys {
// //                     key_hashes.push(generate_sub_key_hash(&wallet, 3, i));
// //                 }
// //             }
// //
// //             let root = redgold::rg_merkle::build_root(&key_hashes, None, &mut None);
// //             println!("Peer ID:");
// //             let hex_peer_id = hex::encode(root);
// //             println!("{}", hex_peer_id);
// //             let mut file = File::create("peer_id").unwrap();
// //             file.write_all(hex_peer_id.as_bytes()).unwrap();
// //         }
// //     }
// //
// //     // more program logic goes here...
// // }
// //
// // // fn main() {
// //
// // // let test_str = "asdf";
// // // let m1 = mnemonic_builder::from_str_rounds(test_str, 10000);
// // // println!("{:?}", m1.to_string());
// // // let h2 = util::dhash_10k_str(test_str);
// // // println!("string bytes {:?}", hex::encode(test_str.as_bytes()));
// // // println!("sha256 1 {:?}", hex::encode(util::dhash_str(test_str)));
// // // println!("sha256 10k {:?}", hex::encode(h2));
// // // let m2 = Mnemonic::new(&h2).unwrap();
// // // println!("{:?}", m2.to_string());
// // // let whitespace = m2.to_string().split_whitespace()
// // //     .collect::<Vec<&str>>().len();
// // // println!("size! {:?}", whitespace);
// //
// // // let hd_path = StandardHDPath::from_str("m/44'/0'/0'/0/0").unwrap();
// // //
// // // let words ="clerk because napkin romance confirm shell fruit diary pilot million pledge monkey vapor dice future message robot crunch junk wheel canal salon can gas";
// // // let mnemonic = Mnemonic::from_str(words).unwrap();
// // //
// // // let mnem_seed = mnemonic.to_seed(None);
// // //
// // // let epk = redgold::wallet::get_pk(&mnem_seed.0, &hd_path);
// // //
// // // println!("epk: {}", epk.private_key.key.to_hex());
// // // }
// fn main() {}
