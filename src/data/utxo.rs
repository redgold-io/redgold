use redgold_schema::structs::Address;
use crate::schema::utxo_id::OldUtxoId;
use crate::schema::structs::{Input, Output, Transaction, UtxoEntry};
use crate::schema::transaction::amount_data;
use crate::schema::{SafeBytesAccess, WithMetadataHashable};
use crate::util;
use redgold_keys::util::mnemonic_words::generate_key;

//
// #[test]
// fn test_conversions() {
//     let value: u32 = 0x1FFFF;
//     let bytes = value.to_le_bytes();
//     let hash = util::sha256_str("asdf");
//     let mut merged = [0u8; 36];
//     merged[0..32].clone_from_slice(&hash);
//     merged[32..36].clone_from_slice(&bytes);
//     let _vec = merged.to_vec();
//     // println!("{:?}" , vec);
//     // println!("{:?}" , hash);
//     // println!("{:?}" , bytes);
//     let res = FixedIdConvert::from_values(&hash, value);
//     let (hash2, value2) = res.to_values();
//     assert_eq!(hash, hash2);
//     assert_eq!(value, value2);
//
//     let res2 = UtxoEntry::id_from_values(&hash.to_vec(), &bytes.to_vec());
//     assert_eq!(res.id.to_vec(), res2);
//     let (hash3, value3) = UtxoEntry::id_to_values(&res2);
//     assert_eq!(hash.to_vec(), hash3);
//     assert_eq!(value3, value);
//     // let res2 = UtxoEntry::from_fixed_values(&hash.to_vec(), &bytes.to_vec());
// }
