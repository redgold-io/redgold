use crate::node_config::EnvDefaultNodeConfig;
use crate::schema::structs::Transaction;
use itertools::Itertools;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::TestConstants;
use redgold_rpc_integ::examples::example::dev_ci_kp;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::constants::{EARLIEST_TIME, MAX_COIN_SUPPLY};
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, Seed};
use redgold_schema::tx::tx_builder::TransactionBuilder;

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
        main_entry("0a220a209dd9790a8d75eb17555d091ffe7ff5e77a6a5b7ca7a0993abb0d80476aa7fe18", 1),
        // 1 - Original dev fund
        main_entry("0a220a2033150611bcbaf033e7ccb2cd90a05432ddc8c3a9cf1ebd8744f3fa93849aa055", 10),
        // 2 - Foundation fund
        main_entry("0a220a201cb5cee74fcbaf6c381e146b9677640801f26b20fee8b78307af3a09915dc3dd", 10),
        // 3 - Future dev fund
        main_entry("0a220a20304c0a4e52fc4c4425b8edb7fdeb016fbd74e65dabdeb2b2d472ebb311638d88", 10),
        // 4 - Anon-N
        main_entry("0a220a20dc60094db8f18f7e408af71d7d1c574397b3d04e05bf7dc6b1c171193cda1c67", 2),
        // 5 - Anon-T
        main_entry("0a220a206765bf4a6a35845f89d489b9c06226a9f1e1a69a16a3d00ebe6402cd834cb6da", 0.5),
        // 6 - Anon-X
        main_entry("0a220a2061fce745ec2041557d210834a8a23d754b182842ea41d1c53e95bd07a23b693f", 0.5),
        // 7 - Anon-J
        main_entry("0a220a20337f2398775b729d5178ff2ed7ce163736e0f343a66d11987b397fae456b8f8e", 0.5),
        // 8 - Anon-R
        main_entry("0a220a2003935fad58b99aa2d2678806e2d0f36df188419e73859ec50d9a0e0c00b87cdb", 0.5),
        // 9 - Origin DAO
        main_entry("0a220a2015559790fd1235640c80421e55422cd91f16c3bd70bcf6e05faab5afe4114aea", 65),
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
#[ignore]
#[tokio::test]
pub async fn verify_genesis_distribution_main() {
    let nc = NodeConfig::dev_default().await;
    let tc = TestConstants::new();
    main_distribution(&tc.address_1, nc.seeds.as_ref());
}

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

    let mut res = pks.iter().map(|o| {
        let distribution = GenesisDistribution {
            address: Address::from_struct_public(o).expect("works"),
            amount: CurrencyAmount::from_fractional((1.0 / pks.len() as f64) * (MAX_COIN_SUPPLY as f64)).expect("works"),
        };
        distribution
    }).collect_vec();

    if let Some((_, kp)) = dev_ci_kp() {
        add_entry_mutate_first(&mut res, &kp.address_typed(), 1000.0);
    }

    let addr = WordsPass::test_words().keypair_at_change(0).expect("works").public_key().address().expect("works");
    add_entry_mutate_first(&mut res, &addr, 1000.0);

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
    let x = txb
        .with_no_salt()
        .with_time(Some(EARLIEST_TIME))
        .with_pow().expect("pow").transaction.clone();
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
