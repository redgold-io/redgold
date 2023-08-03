use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, SecretKey, Signature};
use curv::arithmetic::Zero;
use itertools::Itertools;
use log::{debug, info};
use prost::{DecodeError, Message as msg};
use rusqlite::Connection;
use serde_json;

use crate::e2e::tx_gen::TransactionGenerator;
use crate::core::relay::Relay;
use crate::data::data_store::DataStore;
use crate::schema::structs::{Error as RGError, Hash};
use crate::schema::structs::{ErrorInfo, StructMetadata};
use crate::schema::structs::{
    HashType, Input, ObservationMetadata, Output, Proof, Request, Response, Transaction,
};
use crate::schema::structs::{PeerData, ResponseMetadata, StandardData, UtxoEntry};
use crate::schema::util::{dhash_str, public_key_ser};
use crate::schema::{error_message};
use crate::schema::TestConstants;
use crate::schema::{KeyPair};
use crate::schema::{response_metadata, SafeOption};
use crate::schema::{
    struct_metadata, HashClear, ProtoHashable, SafeBytesAccess, WithMetadataHashable,
    WithMetadataHashableFields,
};
use crate::{genesis, util};
use redgold_schema::constants::{
    DECIMAL_MULTIPLIER, EARLIEST_TIME, MAX_COIN_SUPPLY, MAX_INPUTS_OUTPUTS,
};
use redgold_schema::ProtoSerde;
use redgold_schema::util::mnemonic_words::MnemonicWords;
use crate::schema::transaction::rounded_balance;
// use crate::schema::transaction::rounded_amount;


// TODO: Invest in KeyPair class?

// TODO Proof verification unit tests.

//const GENESIS_HEX_STR =
#[allow(dead_code)]
pub struct TransactionTestContext {
    pub tc: TestConstants,
    pub ds: DataStore,
    pub c: Connection,
    pub g: Transaction,
    pub t: Transaction,
    pub t2: Transaction,
    pub relay: Relay,
    pub tx_gen: TransactionGenerator,
}
//
// impl TransactionTestContext {
//     #[allow(dead_code)]
//     pub async fn default() -> Self {
//         let relay = Relay::default().await;
//         let tc = TestConstants::new();
//         let ds = relay.ds.clone();
//         info!("Data store path in test context {}", ds.connection_path);
//         let c = ds
//             .create_all_err_info()
//             //.await
//             .expect("create");
//         let g = genesis::create_genesis_transaction();
//
//         ds.transaction_store.insert_transaction(&g, EARLIEST_TIME, true, None)
//             .await.expect("Insert fail");
//         info!(
//             "Data store immediate genesis query {:?}",
//             ds.transaction_store.utxo_all_debug().await.unwrap()
//         );
//
//         let tx_gen = TransactionGenerator::default(vec![]).with_genesis();
//
//         let vec = g.to_utxo_entries(0 as u64);
//         let source = vec.get(0).unwrap();
//         let t = Transaction::new(source, &tc.addr, 20000, &tc.secret, &tc.public);
//         let t2 = Transaction::new(source, &tc.addr2, 20000, &tc.secret, &tc.public);
//         Self {
//             tc,
//             ds,
//             c,
//             g,
//             t,
//             t2,
//             relay,
//             tx_gen,
//         }
//     }
// }

// Re-enable later.
// // TODO: assertions about bad proofs and insufficent balance and so on.
// #[test]
// fn test_validation() {
//     let ttc = TransactionTestContext::default();
//     let t = ttc.t;
//     let result = t.validate_utxo(&ttc.ds);
//     assert!(result.is_ok());
//
//     let mut t_invalid_sig = t.clone();
//     let p2 = Proof::new(&ttc.tc.hash_vec, &ttc.tc.secret2, &ttc.tc.public);
//     t_invalid_sig
//         .inputs
//         .get_mut(0)
//         .unwrap()
//         .proof
//         .get_mut(0)
//         .unwrap()
//         .signature = p2.signature;
//     assert!(t_invalid_sig.validate_utxo(&ttc.ds).is_err());
//
//     let mut t_invalid_amount = t.clone();
//     t_invalid_amount.outputs.get_mut(0).unwrap().amount() == redgold_to_amount(REWARD_AMOUNT * 100);
//     assert!(t_invalid_amount.validate_utxo(&ttc.ds).is_err());
// }

#[test]
fn test_decoding() {
    let t = genesis::create_genesis_transaction();
    let bytes = t.proto_serialize();
    let t2 = Transaction::proto_deserialize(bytes).unwrap();
    assert_eq!(t, t2);
    assert_eq!(t.hash_or(), t2.hash_or());
}

//
// pub fn as_base64<S>(key: &Vec<u8>, serializer: &mut S) -> Result<(), S::Error>
// where
//     S: Serializer,
// {
//     serializer.serialize_str(&base64::encode(key))?;
//     Ok(())
// }
//
// pub fn from_base64<'a, D>(deserializer: &mut D) -> Result<Vec<u8>, D::Error>
// where
//     &mut D: Deserializer<'a>,
// {
//     use serde::de::Error;
//     let res = String::deserialize(deserializer)
//         .and_then(|string| base64::decode(&string).map_err(|err| Error::custom(err.to_string())))
//         .and_then(|opt| opt.ok_or_else(|| Error::custom("failed to deserialize public key")))?;
//     Ok(res)
// }
// https://github.com/serde-rs/serde/issues/661

#[test]
fn test_serialization() {
    let t = genesis::create_genesis_transaction();
    let j = serde_json::to_string(&t.clone()).unwrap();
    let j2 = serde_json::to_string_pretty(&t.clone()).unwrap();
    println!("{}", j2);
    let t2 = serde_json::from_str::<Transaction>(&j).unwrap();
    let test: i32 = 5;
    let test2 = &test;
    println!("{:?}", i32::is_zero(test2));
    assert_eq!(t, t2);
}

#[test]
fn test_serialization_obs() {
    let t = ObservationMetadata {
        observed_hash: Some(dhash_str("asdf").to_vec().into()),
        state: None,
        validation_confidence: None,
        struct_metadata: None,
        observation_type: 0
    };
    let j = serde_json::to_string(&t).unwrap();
    println!("{}", j);
    let t2 = serde_json::from_str::<ObservationMetadata>(&j).unwrap();
    assert_eq!(t, t2);
}
