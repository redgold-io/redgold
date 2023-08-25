use itertools::Itertools;
use crate::core::transaction;
use crate::schema::output::output_data;
use crate::schema::structs::{Block, Output, Transaction, UtxoEntry};
use crate::schema::transaction::amount_data;
use redgold_keys::TestConstants;
use redgold_keys::transaction_support::TransactionBuilderSupport;
use crate::schema::{struct_metadata, WithMetadataHashable};
use redgold_schema::{constants, ProtoHashable};
use redgold_schema::constants::{EARLIEST_TIME, REWARD_AMOUNT};
use redgold_schema::output::tx_output_data;
use redgold_schema::structs::{Address, BlockMetadata, CurrencyAmount};
use redgold_schema::transaction_builder::TransactionBuilder;
//
// pub fn genesis_from_values_amount(hash: &Vec<u8>, amount: u64) -> UtxoEntry {
//     return UtxoEntry {
//         transaction_hash: hash.clone(),
//         address: vec![],
//         output: Some(Output {
//             address: None,
//             product_id: None,
//             counter_party_proofs: vec![],
//             data: Some(amount_data(amount)),
//             contract: None,
//         }),
//         output_index: 0,
//         time: EARLIEST_TIME,
//     };
// }
//
// pub fn create_genesis_transaction() -> Transaction {
//     let tc = TestConstants::new();
//     let entry = genesis_from_values_amount(
//         &tc.hash_vec,
//         transaction::amount_to_raw_amount(REWARD_AMOUNT),
//     );
//     Transaction::new(&entry, &tc.addr, REWARD_AMOUNT, &tc.secret, &tc.public)
// }

pub struct GenesisDistribution{
    pub address: Address,
    pub amount: u64,
}

pub fn genesis_tx_from(distribution: Vec<GenesisDistribution>) -> Transaction {
    let mut txb = TransactionBuilder::new();
    for x in distribution {
        txb.with_output(&x.address, &CurrencyAmount::from_fractional(x.amount as f64).expect("works"));
    }
    let mut options = txb.transaction.options.as_mut().expect("");
    options.salt = None;
    let mut sm = txb.transaction.struct_metadata.as_mut().expect("smd");
    sm.time = Some(EARLIEST_TIME);
    txb.transaction.with_hash().clone()
}

pub fn create_genesis_transaction() -> Transaction {
    let tc = TestConstants::new();
    Transaction {
        inputs: vec![],
        outputs: vec![output_data(tc.addr, REWARD_AMOUNT as u64)],
        struct_metadata: struct_metadata(constants::EARLIEST_TIME as i64),
        options: None
    }
    .with_hash()
    .clone()
}

pub fn create_genesis_block() -> Block {
    let mut b = Block {
        merkle_root: None,
        transactions: vec![create_genesis_transaction()],
        struct_metadata: struct_metadata(constants::EARLIEST_TIME as i64),
        previous_block_hash: None,
        metadata: None,
        hash: None,
        height: 0,
    };
    b.with_hash();
    b.previous_block_hash = b.hash.clone();
    b.clone()
}
