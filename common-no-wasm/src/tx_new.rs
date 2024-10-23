use rand::Rng;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::struct_metadata_new;
use redgold_schema::structs::{Transaction, TransactionOptions, TransactionType};
use redgold_schema::tx::tx_builder::TransactionBuilder;

pub trait TxRandomSaltNew {
    fn new_blank() -> Self;
}

impl TxRandomSaltNew for Transaction {

    fn new_blank() -> Self {
        let mut rng = rand::thread_rng();
        let mut tx = Self::default();
        tx.struct_metadata = struct_metadata_new();

        let mut opts = TransactionOptions::default();
        opts.salt = Some(rng.gen::<i64>());
        opts.transaction_type = TransactionType::Standard as i32;
        tx.options = Some(opts);
        tx
    }
}


pub trait TransactionBuilderSupport {
    fn new(network: &NodeConfig) -> Self;
}

impl TransactionBuilderSupport for TransactionBuilder {
    fn new(config: &NodeConfig) -> Self {
        let tx = Transaction::new_blank();
        let network = config.network.clone();
        let fee_addrs = config.seed_peer_addresses();
        let mut s = Self {
            transaction: tx,
            utxos: vec![],
            used_utxos: vec![],
            used_utxo_ids: vec![],
            network: Some(network.clone()),
            nc: Some(config.clone()),
            fee_addrs,
            allow_bypass_fee: false,
            input_addresses: vec![],
        };
        s.with_network(&network);
        s
    }
}
