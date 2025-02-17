// use crate::genesis::create_test_genesis_transaction;
use crate::schema::structs::{Transaction, UtxoEntry};
use itertools::Itertools;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::transaction_support::{InputSupport, TransactionSupport};
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::KeyPair;
use redgold_keys::TestConstants;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, OutputType, TestContractRequest, UtxoId};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use std::hash::Hash;
use tracing::info;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;

#[derive(Clone, PartialEq)]
pub struct SpendableUTXO {
    pub utxo_entry: UtxoEntry,
    pub key_pair: KeyPair,
}

#[derive(Clone)]
pub struct TransactionWithKey {
    pub transaction: Transaction,
    pub key_pairs: Vec<KeyPair>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TransactionGenerator {
    // runtime: Arc<Runtime>,
    pub finished_pool: Vec<SpendableUTXO>,
    pending_pool: Vec<SpendableUTXO>,
    offset: usize,
    min_offset: usize,
    max_offset: usize,
    pub wallet: WordsPass, // default_client: Option<PublicClient>
    node_config: NodeConfig,
    pub used_utxos: Vec<UtxoId>,
    pub pop_finished: Vec<SpendableUTXO>
}

impl TransactionGenerator {

    pub fn tx_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(&self.node_config)
    }

    pub fn used_spendable(&self) -> Vec<SpendableUTXO> {
        self.pop_finished.iter()
            .filter(|x| !self.finished_pool.contains(x))
            .cloned()
            .collect_vec()
    }

    // pub fn with_genesis(&mut self) -> TransactionGenerator {
    //     let vec = create_test_genesis_transaction()
    //         .to_utxo_entries(0 as u64)
    //         .clone();
    //     let kp = TestConstants::new().key_pair();
    //     for entry in vec {
    //         self.finished_pool.push(SpendableUTXO {
    //             utxo_entry: entry,
    //             key_pair: kp,
    //         });
    //     }
    //     self.clone()
    // }
    pub fn default(utxos: Vec<SpendableUTXO>, node_config: &NodeConfig) -> Self {
        Self {
            finished_pool: utxos,
            pending_pool: vec![],
            offset: 1,
            min_offset: 1,
            max_offset: 49,
            wallet: TestConstants::new().words_pass,
            node_config: node_config.clone(),
            used_utxos: vec![],
            pop_finished: vec![],
        }
    }

    pub fn next_kp(&mut self) -> KeyPair {
        let kp = self.wallet.keypair_at_change(self.offset as i64).expect("keypair");
        self.offset += 1;
        if self.offset >= self.max_offset {
            self.offset = self.min_offset;
        }
        kp
    }

    pub fn all_value_transaction(&mut self, prev: SpendableUTXO) -> TransactionWithKey {
        let kp = self.next_kp();
        let kp2 = kp.clone();


        let entry = prev.utxo_entry;
        let prev_addr_typed = entry.address().unwrap();
        let prev_addr = prev_addr_typed.render_string().unwrap();
        // info!("all_value_transaction prev_addr {prev_addr}");
        let prev_kp = prev.key_pair.address_typed().render_string().unwrap();
        // info!("all_value_transaction prev_kp {prev_kp}");
        let tx = TransactionBuilder::new(&self.node_config)
            .with_utxo(&entry.clone()).expect("Failed to build transaction")
            .with_output(&kp.address_typed(), &CurrencyAmount::from(entry.amount() as i64))
            .build().expect("Failed to build transaction")
            .sign(&prev.key_pair).expect("signed");
        // for i in tx.inputs.iter() {
        //     // info!("Verifying proof with tx hash {}", tx.hash_hex());
        //     i.verify_proof(prev_addr_typed, &tx.signable_hash()).expect("works");
        // }
        TransactionWithKey {
            transaction: tx,
            key_pairs: vec![kp2],
        }
    }

    pub fn all_value_transaction_to(&mut self, prev: SpendableUTXO, to: &Address) -> Transaction {

        let tx = TransactionBuilder::new(&self.node_config)
            .with_utxo(&prev.utxo_entry.clone()).expect("Failed to build transaction")
            .with_output(to, &CurrencyAmount::from(prev.utxo_entry.amount() as i64))
            .build().expect("Failed to build transaction")
            .sign(&prev.key_pair).expect("signed");
        tx
    }


    pub async fn generate_deploy_test_contract(&mut self) -> RgResult<TransactionWithKey> {
        let prev = self.pop_finished().safe_get()?.clone();
        let bytes = tokio::fs::read("../../crates/sdk/test_contract_guest.wasm").await.error_info("Read failure")?;
        let mut tb = TransactionBuilder::new(&self.node_config);
        let x = &prev.utxo_entry;
        tb.with_unsigned_input(x.clone())?;
        let a = x.opt_amount().expect("a");
        let c_amount = CurrencyAmount::from(a.amount / 2);
        // TODO: Add fees / fee address, use genesis utxos or something?
        // let fee_amount = CurrencyAmount::from(a.amount / 10);
        tb.with_contract_deploy_output_and_predicate_input(bytes, c_amount, true)?;
        // tb.with_fee(fee_amount);
        tb.with_remainder();
        let tx= tb.build()?.sign(&prev.key_pair)?;
        let tk = TransactionWithKey {
            transaction: tx,
            key_pairs: vec![prev.key_pair.clone()],
        };
        Ok(tk)
    }

    pub async fn generate_deploy_test_contract_request(&mut self, address: Address) -> RgResult<TransactionWithKey> {
        let prev = self.pop_finished().safe_get()?.clone();
        let mut tb = TransactionBuilder::new(&self.node_config);
        let x = &prev.utxo_entry;
        tb.with_unsigned_input(x.clone())?;
        let a = x.opt_amount().expect("a");
        let _c_amount = CurrencyAmount::from(a.amount / 2);
        // TODO: Add fees / fee address, use genesis utxos or something?
        // let fee_amount = CurrencyAmount::from(a.amount / 10);

        let mut req = TestContractRequest::default();
        let mut update = structs::TestContractUpdate::default();
        update.key = "ASDF".to_string();
        update.value = "omg".to_string();
        let mut update2 = structs::TestContractUpdate2::default();
        update2.value = "TEST UPDATED".to_string();
        req.test_contract_update = Some(update);
        req.test_contract_update2 = Some(update2);

        tb.with_contract_request_output(&address, &req.proto_serialize())?;
        // tb.with_fee(fee_amount);
        tb.with_remainder();
        let tx= tb.build()?.sign(&prev.key_pair)?;
        let tk = TransactionWithKey {
            transaction: tx,
            key_pairs: vec![prev.key_pair.clone()],
        };
        Ok(tk)
    }

    pub fn split_value_transaction(&mut self, prev: &SpendableUTXO) -> TransactionWithKey {
        let kp = self.next_kp();
        let kp2 = kp.clone();
        let amount = prev.utxo_entry.amount() / 2;
        let tx = TransactionBuilder::new(&self.node_config)
            .with_utxo(&prev.utxo_entry.clone()).expect("Failed to build transaction")
            .with_output(&kp.address_typed(), &CurrencyAmount::from(amount as i64))
            .with_output(&kp2.address_typed(), &CurrencyAmount::from(amount as i64))
            .build().expect("Failed to build transaction")
            .sign(&prev.key_pair).expect("signed");

        TransactionWithKey {
            transaction: tx,
            key_pairs: vec![kp2, prev.key_pair],
        }
    }

    pub fn generate_simple_tx(&mut self) -> Result<TransactionWithKey, ErrorInfo> {
        // TODO: This can cause a panic
        let prev = self.pop_finished().safe_get()?.clone();
        let pk = prev.key_pair.public_key();
        let pk_hex = pk.hex();
        let pk_json = pk.json_or();
        let address_prev_utxo = prev.utxo_entry.address().expect("address").render_string().expect("works");
        let pk_addr = pk.address().expect("address").render_string().expect("works");
        info!("generate simple tx pk hex {pk_hex}");
        info!("generate simple tx pk json {pk_json}");
        info!("generate simple tx address_prev_utxo {address_prev_utxo}");
        info!("generate simple tx pk_addr {pk_addr}");
        let key = self.all_value_transaction(prev.clone());
        use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
        // info!("Generate simple TX from utxo hash: {}", hex::encode(prev.clone().utxo_entry.transaction_hash.clone()));
        // info!("Generate simple TX from utxo output_id: {}", prev.clone().utxo_entry.output_index.clone().to_string());
        // info!("Generate simple TX hash: {}", key.transaction.hash_hex_or_missing());
        Ok(key)
    }

    pub fn generate_simple_tx_to(&mut self, address: &Address) -> Result<Transaction, ErrorInfo> {
        // TODO: This can cause a panic
        let prev = self.pop_finished().safe_get()?.clone();
        let pk = prev.key_pair.public_key();
        let pk_hex = pk.hex();
        let pk_json = pk.json_or();
        let address_prev_utxo = prev.utxo_entry.address().expect("address").render_string().expect("works");
        let pk_addr = pk.address().expect("address").render_string().expect("works");
        // info!("generate simple tx pk hex {pk_hex}");
        // info!("generate simple tx pk json {pk_json}");
        // info!("generate simple tx address_prev_utxo {address_prev_utxo}");
        // info!("generate simple tx pk_addr {pk_addr}");
        let key = self.all_value_transaction_to(prev.clone(), &address);
        // info!("Generate simple TX from utxo hash: {}", hex::encode(prev.clone().utxo_entry.transaction_hash.clone()));
        // info!("Generate simple TX from utxo output_id: {}", prev.clone().utxo_entry.output_index.clone().to_string());
        // info!("Generate simple TX hash: {}", key.transaction.hash_hex_or_missing());
        Ok(key)
    }

    pub fn pop_finished(&mut self) -> Option<SpendableUTXO> {
        let spendable = self.finished_pool.pop();
        self.pop_finished.push(spendable.clone().unwrap());
        spendable
    }

    pub fn generate_simple_used_utxo_tx_otherwise_valid(&mut self) -> Result<TransactionWithKey, ErrorInfo> {
        // TODO: This can cause a panic
        let used = self.used_spendable();
        let prev = used.get(0).expect("works").clone();
        let key = self.all_value_transaction(prev.clone());
        // info!("Generate simple TX from utxo hash: {}", hex::encode(prev.clone().utxo_entry.transaction_hash.clone()));
        // info!("Generate simple TX from utxo output_id: {}", prev.clone().utxo_entry.output_index.clone().to_string());
        // info!("Generate simple TX hash: {}", key.transaction.hash_hex_or_missing());
        Ok(key)
    }

    pub fn drain_tx(&mut self, addr: &Address) -> Transaction {
        let prev: SpendableUTXO = self.pop_finished().unwrap();
        // TODO: Fee?
        let txb = TransactionBuilder::new(&self.node_config)
            .with_utxo(&prev.utxo_entry.clone()).expect("Failed to build transaction")
            .with_output(addr, &CurrencyAmount::from( prev.utxo_entry.amount() as i64))
            .build().expect("Failed to build transaction")
            .sign(&prev.key_pair).expect("signed");
        txb
        // use redgold_schema::WithMetadataHashable;
        // info!("Generate simple TX from utxo hash: {}", hex::encode(prev.clone().utxo_entry.transaction_hash.clone()));
        // info!("Generate simple TX from utxo output_id: {}", prev.clone().utxo_entry.output_index.clone().to_string());
        // info!("Generate simple TX hash: {}", key.transaction.hash_hex_or_missing());
        // key
    }

    pub fn generate_split_tx(&mut self) -> Vec<TransactionWithKey> {
        let vec = self.finished_pool.clone();
        self.finished_pool.clear();
        vec.iter()
            .map(|x| self.split_value_transaction(x))
            .collect()
    }

    pub fn generate_double_spend_tx(&mut self) -> (TransactionWithKey, TransactionWithKey) {
        let prev: SpendableUTXO = self.pop_finished().unwrap();
        let tx1 = self.all_value_transaction(prev.clone());
        let tx2 = self.all_value_transaction(prev);
        (tx1, tx2)
    }

    pub fn completed(&mut self, tx: TransactionWithKey) {
        let used_utxos = tx.transaction.fixed_utxo_ids_of_inputs().expect("fixed_utxo_ids_of_inputs");
        let vec = tx.transaction.to_utxo_entries(0 as u64);
        let iter = vec.iter().filter(|v| {
            !v.output.as_ref().and_then(|o| o.output_type.as_ref()).filter(|t| **t == OutputType::Fee as i32)
                .is_some()
        });
        for (i, v) in iter.enumerate() {
            let spendable_utxo = SpendableUTXO {
                utxo_entry: v.clone(),
                key_pair: tx.key_pairs.get(i).or(tx.key_pairs.get(0)).unwrap().clone(),
            };
            self.finished_pool.push(spendable_utxo.clone());
        }
        self.used_utxos.extend(used_utxos);
    }
}

#[ignore]
#[test]
fn verify_signature() {
    let _tc = TestConstants::new();
    // let mut tx_gen = TransactionGenerator::default(vec![], &NetworkEnvironment::Debug) .with_genesis();
    // let tx = tx_gen.generate_simple_tx().expect("");
    // let transaction = create_test_genesis_transaction();
    // let vec1 = transaction.to_utxo_entries(0);
    // let entry = vec1.get(0).expect("entry");
    // let result = tx.transaction.verify_utxo_entry_proof(entry);
    // println!(
    //     "{:?}",
    //     result
    //         .clone()
    //         .map_err(|e| serde_json::to_string(&e).unwrap_or("json".to_string()))
    //         .err()
    //         .unwrap_or("success".to_string())
    // );
    // assert!(result.is_ok());
}
