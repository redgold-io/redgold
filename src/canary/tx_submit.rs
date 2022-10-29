use crate::api::public_api::{PublicClient};
use crate::canary::tx_gen::{SpendableUTXO, TransactionGenerator, TransactionWithKey};
use crate::schema::structs::{Error, PublicResponse, ResponseMetadata, Transaction};
use crate::schema::WithMetadataHashable;
use log::{error, info};
use std::borrow::Borrow;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use redgold_schema::empty_public_response;
use redgold_schema::util::wallet::Wallet;

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
        if utxos.is_empty() {
            generator = generator.with_genesis().clone();
        }
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
        if utxos.is_empty() {
            generator = generator.with_genesis().clone();
        }
        Self {
            generator: Arc::new(Mutex::new(generator)),
            runtime,
            client,
        }
    }

    fn spawn(&self, transaction: Transaction) -> JoinHandle<PublicResponse> {
        let res = self.runtime.spawn({
            let c = self.client.clone();
            let tx = transaction.clone();
            async move {
                info!(
                    "Attemping to spawn test transaction {:?} with client",
                    tx.clone().hash_hex_or_missing()
                );
                let result = c.clone().send_transaction(&tx.clone(), true).await;
                if result.is_err() {
                    let err = result.unwrap_err();
                    error!(
                        "Error on transaction {:?} response: {:?}",
                        tx.clone().hash_hex_or_missing(),
                        err.clone()
                    );
                    let mut response = empty_public_response();
                    response.response_metadata = Some(ResponseMetadata {
                            success: false,
                            error_info: Some(err),
                        });
                    response
                } else {
                    let res = result.unwrap();
                    info!(
                        "Success on transaction {:?} response: {:?}",
                        tx.clone().hash_hex_or_missing(),
                        res.clone()
                    );
                    res
                }
            }
        });
        res
    }

    fn spawn_client(
        &self,
        transaction: Transaction,
        second_client: Option<PublicClient>,
    ) -> JoinHandle<PublicResponse> {
        let res = self.runtime.spawn({
            let c = second_client.unwrap_or(self.client.clone());
            let tx = transaction.clone();
            async move {
                info!("Attemping to spawn test transaction with client");
                c.clone().send_transaction(&tx.clone(), true).await.unwrap()
            }
        });
        res
    }

    pub(crate) fn submit(&self) -> PublicResponse {
        let transaction = self.generator.lock().unwrap().generate_simple_tx().clone();
        let res = self.block(self.spawn(transaction.clone().transaction));
        if res.clone().accepted() {
            self.generator.lock().unwrap().completed(transaction);
        }
        res
    }

    // TODO: make interior here a function
    pub(crate) fn submit_split(&self) -> Vec<PublicResponse> {
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

    pub(crate) fn submit_duplicate(&self) -> Vec<PublicResponse> {
        let transaction = self.generator.lock().unwrap().generate_simple_tx().clone();
        let h1 = self.spawn(transaction.clone().transaction);
        let h2 = self.spawn(transaction.clone().transaction);
        let dups = self.await_results(vec![(h1, transaction.clone()), (h2, transaction.clone())]);
        info!("{}", serde_json::to_string(&dups.clone()).unwrap());
        assert!(dups.iter().any(|x| x.accepted()));
        assert!(dups.iter().any(|x| !x.accepted()));
        assert!(dups.iter().any(|x| x
            .error_code()
            .filter(|x| x == &(Error::TransactionAlreadyProcessing as i32))
            .is_some()));
        dups
    }

    fn await_results(
        &self,
        handles: Vec<(JoinHandle<PublicResponse>, TransactionWithKey)>,
    ) -> Vec<PublicResponse> {
        let mut results: Vec<PublicResponse> = vec![];
        for (h, transaction) in handles {
            let res = self.block(h);
            if res.clone().accepted() {
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
    ) -> Vec<PublicResponse> {
        let (t1, t2) = self
            .generator
            .lock()
            .unwrap()
            .generate_double_spend_tx()
            .clone();
        let h1 = self.spawn(t1.clone().transaction);
        let h2 = self.spawn_client(t2.clone().transaction, second_client);
        let doubles = self.await_results(vec![(h1, t1.clone()), (h2, t2.clone())]);
        info!("{}", serde_json::to_string(&doubles.clone()).unwrap());

        assert!(doubles.iter().any(|x| x.accepted()));
        let one_rejected = doubles.iter().any(|x| !x.accepted());
        // if !one_rejected {
        //     show_balances()
        // }
        assert!(one_rejected);
        assert!(doubles.iter().any(|x| x
            .error_code() // Change to query response? or submit response?
            .filter(|x| x == &(Error::TransactionRejectedDoubleSpend as i32))
            .is_some()));
        doubles
    }

    fn block(&self, jh: JoinHandle<PublicResponse>) -> PublicResponse {
        self.runtime.block_on(jh).unwrap()
    }
}
