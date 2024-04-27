use itertools::Itertools;
use redgold_keys::address_support::AddressSupport;
use crate::schema::structs::Transaction;
use redgold_keys::TestConstants;
use crate::core::transact::tx_builder_supports::TransactionBuilderSupport;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::constants::{DECIMAL_MULTIPLIER, EARLIEST_TIME, MAX_COIN_SUPPLY};
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, Seed};
use crate::core::transact::tx_builder_supports::TransactionBuilder;
use crate::node_config::NodeConfig;

pub struct GenesisDistribution{
    pub address: Address,
    pub amount: CurrencyAmount,
}


fn main_entry(address: impl Into<String>, fraction_pct: impl Into<f64>) -> GenesisDistribution {
    GenesisDistribution {
        address: address.into().parse_address().expect("works"),
        amount: CurrencyAmount::from_fractional((fraction_pct.into() / 100.0) * (MAX_COIN_SUPPLY as f64)).expect("works"),
    }
}

fn add_entry_mutate_first(entries: &mut Vec<GenesisDistribution>, address: &Address, amount: impl Into<f64>) {
    let amount = CurrencyAmount::from_fractional(amount.into()).expect("works");
    entries[0].amount.amount -= amount.amount;
    entries.push(GenesisDistribution {
        address: address.clone(),
        amount
    });
}

fn main_distribution(test_address: &Address, seeds: &Vec<Seed>) -> Vec<GenesisDistribution> {
    let mut entries = vec![
        // TODO: Update these
        // 0 - Active dev fund
        main_entry("3a299a25abcc604983dcabbf8a20dfb1440d6c36766762c936030ee8de6a7465", 1),
        // 1 - Original dev fund
        main_entry("e1234f3be30667f1b8860c1a2bbbd12846f8f4581857f883c825be40e43e9a03", 10),
        // 2 - Foundation fund
        main_entry("04f25fb391f7c59bcc1370115787c49fd0762ca44ca54078dd48e67cd56abe55", 10),
        // 3 - Future dev fund
        main_entry("2d064069d1a012698b6791e783f3b9e1c2c65146bb4408ba8b209e3d61e20924", 10),
        // 4 - Anon-N
        main_entry("282111c64b7da428f75ec3b8fcfda186e164e18c597c246f9ffb1e16cbc42729", 2),
        // 5 - Anon-T
        main_entry("0b3ab3c3ed000de6d39db543aff29c885bafe50010111c9f7713001a974e9961", 0.5),
        // 6 - Anon-X
        main_entry("7d220dea6f6854572d7d82d9923fb6e677bd1968f0df7fe4c70b743e9445984e", 0.5),
        // 7 - Anon-J
        main_entry("0bc3af2b862e75e69eb59bd5d4354544f76da9269c5354821afb9c450079d9a4", 0.5),
        // 8 - Anon-R
        main_entry("91f7158f3b6aee0697288ed8b4c7b3ba782d70dede85a7b9322aed42e16e814d", 0.5),
        // 9 - Origin DAO
        main_entry("8965cf0387275d2ac5100b9a3d0e46d9d5cf6e6066db9d5779b1f1649f159068", 65),
        // Node testing address
        GenesisDistribution { address: test_address.clone(), amount: CurrencyAmount::from_fractional(10.0).expect("a") }
    ];

    add_entry_mutate_first(&mut entries, test_address, 10.0);

    seeds.iter().for_each(|s| {
        if let Some(addr) = s.public_key.as_ref().and_then(|pk| pk.address().ok()) {
            add_entry_mutate_first(&mut entries, &addr, 5.0);
        }
        if let Some(addr) = s.peer_id.as_ref()
            .and_then(|pk| pk.peer_id.as_ref())
            .and_then(|pk| pk.address().ok()) {
            add_entry_mutate_first(&mut entries, &addr, 5.0);
        }
    });

    // Debug hot addresses.

    let total = entries.iter().map(|e| e.amount.to_rounded_int()).sum::<i64>();
    assert_eq!(total, MAX_COIN_SUPPLY);

    entries
}
// #[test]
// pub fn verify_genesis_distribution_main() {
//     let tc = TestConstants::new();
//     main_distribution(&tc.address_1);
// }

fn lower_distribution(_network: &NetworkEnvironment, words_pass: &WordsPass, seeds: &Vec<Seed>) -> Vec<GenesisDistribution> {
    let mut pks = vec![];

    for i in 0..50 {
        let pk = words_pass.keypair_at_change(i).expect("works").public_key();
        pks.push(pk);
    }

    for s in seeds.iter() {
        if let Some(pk) = &s.public_key {
            if !pks.contains(pk) {
                pks.push(pk.clone());
            }
        }
    }

    let res = pks.iter().map(|o| {
        GenesisDistribution {
            address: Address::from_struct_public(o).expect("works"),
            amount: CurrencyAmount::from_fractional((1.0 / pks.len() as f64) * (MAX_COIN_SUPPLY as f64)).expect("works"),
        }
    }).collect_vec();
    res
}

pub fn genesis_transaction(
    nc: &NodeConfig,
    words: &WordsPass,
    seeds: &Vec<Seed>
) -> Transaction {
    let distribution = if nc.network.is_main_stage_network() {
        main_distribution(&words.default_public_key().expect("default_kp").address().expect("address"), seeds)
    } else {
        lower_distribution(&nc.network, words, seeds)
    };
    genesis_tx_from(distribution, nc)
}


pub fn genesis_tx_from(distribution: Vec<GenesisDistribution>, network: &NodeConfig) -> Transaction {
    let mut txb = TransactionBuilder::new(network);
    for d in distribution {
        txb.with_output(&d.address, &d.amount);
    }
    let x = txb.with_no_salt().with_time(Some(EARLIEST_TIME))
        .transaction.with_hashes();
    x.clone()
}

// pub fn create_test_genesis_transaction() -> Transaction {
//     let tc = TestConstants::new();
//     Transaction {
//         inputs: vec![],
//         outputs: vec![output_data(tc.addr, REWARD_AMOUNT as u64)],
//         struct_metadata: struct_metadata(constants::EARLIEST_TIME as i64),
//         options: None
//     }
//     .with_hash()
//     .clone()
// }
//
// // This is out of date.
// pub fn create_genesis_block() -> Block {
//     let mut b = Block {
//         merkle_root: None,
//         transactions: vec![create_test_genesis_transaction()],
//         struct_metadata: struct_metadata(constants::EARLIEST_TIME as i64),
//         previous_block_hash: None,
//         metadata: None,
//         height: 0,
//     };
//     b.with_hash();
//     b.previous_block_hash = Some(b.hash_or());
//     b.clone()
// }
