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

pub fn validate_utxo(
    transaction: &Transaction,
    data_store: &DataStore,
) -> Result<Vec<(Vec<u8>, i64)>, ErrorInfo> {
    // TODO: Add errors around unsupported types.
    // let contracts = self
    //     .outputs
    //     .iter()
    //     .map(|o| o.contract.clone())
    //     .collect::<HashSet<Vec<u8>>>();
    // if contracts.len() > 1 {
    //     return Err(RGError::UnknownError);
    // }
    // let contract = contracts.iter().next().unwrap().clone();
    //
    // if contract == Transaction::currency_contract_hash() {
    //     return self.validate_currency_utxo(data_store);
    // }
    return validate_currency_utxo(transaction, data_store);
    // Err(RGError::UnknownError)
}
pub fn delete_utxo_inputs(transaction: &Transaction, ds: DataStore) -> Result<(), rusqlite::Error> {
    for (x, y) in transaction.iter_utxo_inputs() {
        ds.delete_utxo(&x, y as u32)?;
    }
    Ok(())
}

pub fn validate_currency_utxo(
    transaction: &Transaction,
    data_store: &DataStore,
) -> Result<Vec<(Vec<u8>, i64)>, ErrorInfo> {
    // Validate all UTXO's present
    let mut balance: u64 = 0;
    let vec = transaction.iter_utxo_inputs();
    // .iter() instead?
    for input in transaction.inputs.clone() {

        // TODO: change to utxo_id and update the query func to be sqlx.
        let hash = input.transaction_hash.safe_bytes()?.clone();
        let output_id = input.output_index;
    // for (hash, output_id) in vec.clone() {
        // TODO: How to handle DB failure here?
        let utxo_id_hex = hex::encode(hash.clone()) + " " + &*format!("{:?}", output_id);
        let query_result = data_store.query_utxo(&hash, output_id as u32).unwrap();
        match query_result {
            None => {
                log::debug!("Unknown utxo id: {}", utxo_id_hex);
                log::debug!(
                    "Query all utxo {:?}",
                    data_store.query_utxo_all_debug().unwrap()
                );
                log::debug!("Unknown utxo id: {} output {:?}", utxo_id_hex, output_id);
                // let details = ErrorDetails {
                //     detail_name: "utxo_id".to_string(),
                //     detail: utxo_id_hex,
                // };
                // TODO: Return all sorts of debug information here, including the input serialized etc.
                return Err(error_message(
                    RGError::UnknownUtxo,
                    "Transaction validation func",
                ));
            }
            Some(utxo_entry) => {
                let amount = utxo_entry
                    .output
                    .safe_get_msg("UTXO entry from query transaction validate output")?
                    .amount();
                balance += amount;
                log::debug!(
                    "idx: {:?}, amount: {:?}, balance: {:?}, raw: {:?} UtxoEntry: {}",
                    utxo_id_hex,
                    rounded_balance(amount),
                    rounded_balance(balance),
                    amount,
                    serde_json::to_string(&utxo_entry.clone()).unwrap()
                );
                // let option = transaction.inputs.get(output_id as usize);
                // let input = option.safe_get_msg("Transaction input get output_id empty")?;
                Proof::verify_proofs(
                    &input.proof,
                    &utxo_entry.transaction_hash.into(),
                    &utxo_entry.address.into(),
                )?;
            }
        }
    }
    let desired_spend: u64 = transaction
        .outputs
        .iter()
        .map(|o| {
            let amount = o.amount();
            // info!(
            //     "Desired spend raw: {:?} address: {:?}",
            //     amount,
            //     hex::encode(o.address.clone())
            // );
            amount
        })
        .sum();
    let total = desired_spend;
    log::debug!(
        "balance {:?} desired_spend {:?} total {:?}",
        balance,
        desired_spend,
        total
    );
    if total > balance {
        return Err(error_message(RGError::InsufficientBalance, "total greater than balance"));
    }
    if total != balance {
        return Err(error_message(RGError::BalanceMismatch, "Unused funds"));
    }
    return Ok(vec);
}

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

impl TransactionTestContext {
    #[allow(dead_code)]
    pub async fn default() -> Self {
        let relay = Relay::default().await;
        let tc = TestConstants::new();
        let ds = relay.ds.clone();
        info!("Data store path in test context {}", ds.connection_path);
        let c = ds
            .create_all_err_info()
            //.await
            .expect("create");
        let g = genesis::create_genesis_transaction();

        ds.transaction_store.insert_transaction(&g, EARLIEST_TIME, true, None)
            .await.expect("Insert fail");
        info!(
            "Data store immediate genesis query {:?}",
            ds.query_utxo_all_debug().unwrap()
        );

        let tx_gen = TransactionGenerator::default(vec![]).with_genesis();

        let vec = g.to_utxo_entries(0 as u64);
        let source = vec.get(0).unwrap();
        let t = Transaction::new(source, &tc.addr, 20000, &tc.secret, &tc.public);
        let t2 = Transaction::new(source, &tc.addr2, 20000, &tc.secret, &tc.public);
        Self {
            tc,
            ds,
            c,
            g,
            t,
            t2,
            relay,
            tx_gen,
        }
    }
}

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
