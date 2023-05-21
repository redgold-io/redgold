use crate::api::public_api::{PublicClient};
use crate::canary::tx_gen::{SpendableUTXO, TransactionGenerator, TransactionWithKey};
use crate::schema::structs::{Error, PublicResponse, ResponseMetadata, Transaction};
use crate::schema::WithMetadataHashable;
use log::{error, info};
use std::borrow::Borrow;
use std::ops::Sub;
use std::sync::{Arc, Mutex};
use itertools::Itertools;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use redgold_schema::{empty_public_response, ErrorInfoContext, SafeBytesAccess};
use redgold_schema::structs::{Address, ErrorInfo, FaucetResponse, SubmitTransactionResponse};
use redgold_schema::util::wallet::Wallet;
use crate::core::internal_message::FutLoopPoll;

pub struct TransactionSubmitter {
    pub generator: Arc<Mutex<TransactionGenerator>>,
    runtime: Arc<Runtime>,
    client: PublicClient,
}

impl TransactionSubmitter {
    pub fn default(
        client: PublicClient,
        runtime: Arc<Runtime>,
        utxos: Vec<SpendableUTXO>,
    ) -> Self {
        let mut generator = TransactionGenerator::default(utxos.clone());
        // if utxos.is_empty() {
        //     generator = generator.with_genesis().clone();
        // }
        Self {
            generator: Arc::new(Mutex::new(generator)),
            runtime,
            client,
        }
    }
    pub fn default_adv(
        client: PublicClient,
        runtime: Arc<Runtime>,
        utxos: Vec<SpendableUTXO>,
        min_offset: usize,
        max_offset: usize,
        wallet: Wallet
    ) -> Self {
        let mut generator = TransactionGenerator::default_adv(utxos.clone(), min_offset, max_offset, wallet);
        // if utxos.is_empty() {
        //     generator = generator.with_genesis().clone();
        // }
        Self {
            generator: Arc::new(Mutex::new(generator)),
            runtime,
            client,
        }
    }

    fn spawn(&self, transaction: Transaction) -> JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>> {
        let res = self.runtime.spawn({
            let c = self.client.clone();
            let tx = transaction.clone();
            async move {
                info!(
                    "Attemping to spawn test transaction {:?} with client",
                    tx.clone().hash_hex_or_missing()
                );
                let result = c.clone().send_transaction(&tx.clone(), true).await;
                if result.clone().is_err() {
                    let err = result.clone().unwrap_err();
                    error!(
                        "Error on transaction {:?} response: {:?}",
                        tx.clone().hash_hex_or_missing(),
                        err.clone()
                    );
                    result
                } else {
                    let res = result.clone().unwrap();
                    info!(
                        "Success on transaction {:?} response: {:?}",
                        tx.clone().hash_hex_or_missing(),
                        res.clone()
                    );
                    result
                }
            }
        });
        res
    }

    fn spawn_client(
        &self,
        transaction: Transaction,
        second_client: Option<PublicClient>,
    ) -> JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>> {
        let res = self.runtime.spawn({
            let c = second_client.unwrap_or(self.client.clone());
            let tx = transaction.clone();
            async move {
                info!("Attemping to spawn test transaction with client");
                c.clone().send_transaction(&tx.clone(), true).await
            }
        });
        res
    }

    pub fn submit(&self) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let transaction = self.generator.lock().unwrap().generate_simple_tx().clone();
        let res = self.block(self.spawn(transaction.clone().transaction))?;
        // if res.clone().accepted() {
        self.generator.lock().unwrap().completed(transaction);
        // }
        Ok(res)
    }

    pub fn drain(&self, to: Address) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let transaction = self.generator.lock().unwrap().drain_tx(&to).clone();
        let res = self.block(self.spawn(transaction.clone()));
        res
    }

    pub fn with_faucet(&self) -> FaucetResponse {
        let pc = &self.client;
        let w = Wallet::from_phrase("random").key_at(0);
        let a = w.address_typed();
        let vec_a = a.address.safe_bytes().expect("a");
        let res = self.runtime.block_on(pc.faucet(&a, true)).expect("faucet");
        // let h = res.transaction_hash.expect("tx hash");
        // let q = self.runtime.block_on(pc.query_hash(h.hex())).expect("query hash");
        // let tx_info = q.transaction_info.expect("transaction");
        // assert!(!tx_info.observation_proofs.is_empty());
        // let tx = tx_info.transaction.expect("tx");// TODO: does this matter 0 time?
        let tx = res.clone().transaction.expect("tx");
        let vec = tx.to_utxo_entries(0);
        let vec1 = vec.iter().filter(|u|
            u.address == vec_a).collect_vec();
        let matching_entries = vec1.get(0);
        let utxos = matching_entries.expect("utxo").clone().clone();

        self.generator.lock().unwrap().finished_pool.push(
            SpendableUTXO{
                utxo_entry: utxos,
                key_pair: w
            });
        res
    }

    // TODO: make interior here a function
    pub fn submit_split(&self) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let transaction = self.generator.lock().unwrap().generate_split_tx().clone();
        let mut h = vec![];
        for x in transaction {
            h.push((self.spawn(x.clone().transaction), x));
        }
        self.await_results(h)
    }

    pub fn get_addresses(&self) -> Vec<Vec<u8>> {
        self.generator.lock().unwrap().get_addresses()
    }

    pub(crate) fn submit_duplicate(&self) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let transaction = self.generator.lock().unwrap().generate_simple_tx().clone();
        let h1 = self.spawn(transaction.clone().transaction);
        let h2 = self.spawn(transaction.clone().transaction);
        let dups = self.await_results(vec![(h1, transaction.clone()), (h2, transaction.clone())]);
        info!("{}", serde_json::to_string(&dups.clone()).unwrap());
        assert!(dups.iter().any(|x| x.is_ok()));
        assert!(dups.iter().any(|x| !x.is_err()));
        assert!(dups.iter().any(|x|
            x.clone().err().filter(|e| e.code == Error::TransactionAlreadyProcessing as i32).is_some()
        ));

        dups
    }

    fn await_results(
        &self,
        handles: Vec<(JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>>, TransactionWithKey)>,
    ) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let mut results = vec![];
        for (h, transaction) in handles {
            let res = self.block(h);
            if res.clone().is_ok() {
                self.generator
                    .lock()
                    .unwrap()
                    .completed(transaction.clone());
            }
            results.push(res);
        }
        results
    }

    pub(crate) fn submit_double_spend(
        &self,
        second_client: Option<PublicClient>,
    ) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let (t1, t2) = self
            .generator
            .lock()
            .unwrap()
            .generate_double_spend_tx()
            .clone();
        let h1 = self.spawn(t1.clone().transaction);
        let h2 = self.spawn_client(t2.clone().transaction, second_client);
        let doubles = self.await_results(vec![(h1, t1.clone()), (h2, t2.clone())]);
        info!("Double spend test response: {}", serde_json::to_string(&doubles.clone()).unwrap());

        assert!(doubles.iter().any(|x| x.is_ok()));
        let one_rejected = doubles.iter().any(|x| !x.is_ok());

        // if !one_rejected {
        //     show_balances()
        // }
        assert!(one_rejected);
        // assert!(doubles.iter().any(|x| x
        //     .clone()
        //     .err()
        //     .map(|x| x.code)
        //     .filter(|x| x == &(Error::TransactionRejectedDoubleSpend as i32))
        //     .is_some()));

        doubles
    }

    fn block(&self, jh: JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>>) -> Result<SubmitTransactionResponse, ErrorInfo> {
        self.runtime.block_on(jh).error_info("submit joinhandle error")?
    }
}
