use crate::genesis::create_genesis_transaction;
use crate::schema::structs::{Transaction, UtxoEntry};
use redgold_keys::KeyPair;
use redgold_keys::TestConstants;
use redgold_keys::transaction_support::{TransactionBuilderSupport, TransactionSupport};
use redgold_schema::constants::MIN_FEE_RAW;
use redgold_schema::structs::{Address, AddressType, ErrorInfo, TransactionAmount};
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::transaction_builder::TransactionBuilder;
use redgold_keys::util::mnemonic_words::MnemonicWords;

#[derive(Clone)]
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
    pub wallet: MnemonicWords, // default_client: Option<PublicClient>
}

impl TransactionGenerator {

    pub fn with_genesis(&mut self) -> TransactionGenerator {
        let vec = create_genesis_transaction()
            .to_utxo_entries(0 as u64)
            .clone();
        let kp = TestConstants::new().key_pair();
        for entry in vec {
            self.finished_pool.push(SpendableUTXO {
                utxo_entry: entry,
                key_pair: kp,
            });
        }
        self.clone()
    }
    pub fn default_adv(utxos: Vec<SpendableUTXO>, min_offset: usize, max_offset: usize, wallet: MnemonicWords) -> Self {
        Self {
            finished_pool: utxos,
            pending_pool: vec![],
            offset: min_offset,
            min_offset,
            max_offset,
            wallet
        }
    }
    pub fn default(utxos: Vec<SpendableUTXO>) -> Self {
        Self {
            finished_pool: utxos,
            pending_pool: vec![],
            offset: 1,
            min_offset: 1,
            max_offset: 49,
            wallet: MnemonicWords::test_default()
        }
    }

    pub fn set_wallet(&mut self, wallet: MnemonicWords) {
        self.wallet = wallet;
    }

    pub fn next_kp(&mut self) -> KeyPair {
        let kp = self.wallet.key_at(self.offset);
        self.offset += 1;
        if self.offset >= self.max_offset {
            self.offset = self.min_offset;
        }
        kp
    }

    pub fn all_value_transaction(&mut self, prev: SpendableUTXO) -> TransactionWithKey {
        let kp = self.next_kp();
        let kp2 = kp.clone();
        let tx = Transaction::new(
            &prev.utxo_entry,
            &kp.address(),
            prev.utxo_entry.amount(),
            &prev.key_pair.secret_key,
            &prev.key_pair.public_key,
        );
        TransactionWithKey {
            transaction: tx,
            key_pairs: vec![kp2],
        }
    }


    pub async fn generate_deploy_test_contract(&mut self) -> RgResult<TransactionWithKey> {
        let prev = self.finished_pool.pop().safe_get()?.clone();
        let bytes = tokio::fs::read("test_contract_guest.wasm").await.error_info("Read failure")?;
        let mut tb = TransactionBuilder::new();
        let x = &prev.utxo_entry;
        tb.with_unsigned_input(x.clone())?;
        let a = x.opt_amount().expect("a");
        let c_amount = TransactionAmount::from(a.amount / 2);
        // TODO: Add fees / fee address, use genesis utxos or something?
        // let fee_amount = TransactionAmount::from(a.amount / 10);
        tb.with_contract_output(bytes, c_amount, true)?;
        // tb.with_fee(fee_amount);
        tb.with_remainder();
        let tx= tb.transaction.sign(&prev.key_pair)?;
        let tk = TransactionWithKey {
            transaction: tx,
            key_pairs: vec![prev.key_pair.clone()],
        };
        Ok(tk)
    }

    pub fn split_value_transaction(&mut self, prev: &SpendableUTXO) -> TransactionWithKey {
        let kp = self.next_kp();
        let kp2 = kp.clone();
        let tx = Transaction::new(
            &prev.utxo_entry,
            &kp.address(),
            prev.utxo_entry.amount() / 2,
            &prev.key_pair.secret_key,
            &prev.key_pair.public_key,
        );
        TransactionWithKey {
            transaction: tx,
            key_pairs: vec![kp2, prev.key_pair],
        }
    }

    pub fn generate_simple_tx(&mut self) -> Result<TransactionWithKey, ErrorInfo> {
        // TODO: This can cause a panic
        let prev = self.finished_pool.pop().safe_get()?.clone();
        let key = self.all_value_transaction(prev.clone());
        use redgold_schema::WithMetadataHashable;
        // info!("Generate simple TX from utxo hash: {}", hex::encode(prev.clone().utxo_entry.transaction_hash.clone()));
        // info!("Generate simple TX from utxo output_id: {}", prev.clone().utxo_entry.output_index.clone().to_string());
        // info!("Generate simple TX hash: {}", key.transaction.hash_hex_or_missing());
        Ok(key)
    }

    pub fn drain_tx(&mut self, addr: &Address) -> Transaction {
        let prev: SpendableUTXO = self.finished_pool.pop().unwrap();
        // TODO: Fee?
        let txb = TransactionBuilder::new()
            .with_utxo(&prev.utxo_entry.clone()).expect("Failed to build transaction")
            .with_output(addr, &TransactionAmount::from( prev.utxo_entry.amount() as i64))
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
        let prev: SpendableUTXO = self.finished_pool.pop().unwrap();
        let tx1 = self.all_value_transaction(prev.clone());
        let tx2 = self.all_value_transaction(prev);
        (tx1, tx2)
    }

    pub fn completed(&mut self, tx: TransactionWithKey) {
        let vec = tx.transaction.to_utxo_entries(0 as u64);
        let iter = vec.iter().filter(|v| {
            v.opt_amount().map(|a| a.amount > (MIN_FEE_RAW)).unwrap_or(false)
                && !(v.address.clone().expect("a").address_type == AddressType::ScriptHash as i32)
        });
        for (i, v) in iter.enumerate() {
            self.finished_pool.push(SpendableUTXO {
                utxo_entry: v.clone(),
                key_pair: tx.key_pairs.get(i).or(tx.key_pairs.get(0)).unwrap().clone(),
            });
        }
    }
}

#[test]
fn verify_signature() {
    let _tc = TestConstants::new();
    let mut tx_gen = TransactionGenerator::default(vec![]).with_genesis();
    let tx = tx_gen.generate_simple_tx().expect("");
    let transaction = create_genesis_transaction();
    let vec1 = transaction.to_utxo_entries(0);
    let entry = vec1.get(0).expect("entry");
    let result = tx.transaction.verify_utxo_entry_proof(entry);
    println!(
        "{:?}",
        result
            .clone()
            .map_err(|e| serde_json::to_string(&e).unwrap_or("json".to_string()))
            .err()
            .unwrap_or("success".to_string())
    );
    assert!(result.is_ok());
}
