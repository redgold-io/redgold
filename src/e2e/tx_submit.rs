use std::borrow::Borrow;
use std::ops::Sub;
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use log::{error, info};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use redgold_schema::{empty_public_response, error_info, ErrorInfoContext, RgResult, SafeBytesAccess, SafeOption};
use redgold_schema::structs::{Address, ErrorInfo, FaucetResponse, FixedUtxoId, SubmitTransactionResponse};
use redgold_keys::util::mnemonic_words::MnemonicWords;

use crate::api::public_api::PublicClient;
use crate::e2e::tx_gen::{SpendableUTXO, TransactionGenerator, TransactionWithKey};
use crate::schema::structs::{Error, PublicResponse, ResponseMetadata, Transaction};
use crate::schema::WithMetadataHashable;
use redgold_schema::EasyJson;


pub struct TransactionSubmitter {
    pub generator: Arc<Mutex<TransactionGenerator>>,
    // runtime: Arc<Runtime>,
    client: PublicClient,
}


impl TransactionSubmitter {
    pub fn default(
        client: PublicClient,
        // runtime: Arc<Runtime>,
        utxos: Vec<SpendableUTXO>,
    ) -> Self {
        let generator = TransactionGenerator::default(utxos.clone());
        // if utxos.is_empty() {
        //     generator = generator.with_genesis().clone();
        // }
        Self {
            generator: Arc::new(Mutex::new(generator)),
            // runtime,
            client,
        }
    }
    pub fn default_adv(
        client: PublicClient,
        utxos: Vec<SpendableUTXO>,
        min_offset: usize,
        max_offset: usize,
        wallet: MnemonicWords
    ) -> Self {
        let generator = TransactionGenerator::default_adv(utxos.clone(), min_offset, max_offset, wallet);
        // if utxos.is_empty() {
        //     generator = generator.with_genesis().clone();
        // }
        Self {
            generator: Arc::new(Mutex::new(generator)),
            // runtime,
            client,
        }
    }

    fn spawn(&self, transaction: Transaction) -> JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>> {
        let res = tokio::spawn({
            let c = self.client.clone();
            let tx = transaction.clone();
            async move {
                // info!(
                //     "Attemping to spawn test transaction {} with client",
                //     tx.clone().hash_hex_or_missing()
                // );
                let result = c.clone().send_transaction(&tx.clone(), true).await;
                if result.clone().is_err() {
                    let err = result.clone().unwrap_err();
                    error!(
                        "Error on transaction {} response: {}",
                        tx.clone().hash_hex_or_missing(),
                        err.clone().json_or()
                    );
                    result
                } else {
                    let _ = result.clone().unwrap();
                    // info!(
                    //     "Success on transaction {:?} response: {:?}",
                    //     tx.clone().hash_hex_or_missing(),
                    //     res.clone()
                    // );
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
        let res = tokio::spawn({
            let c = second_client.unwrap_or(self.client.clone());
            let tx = transaction.clone();
            async move {
                info!("Attemping to spawn test transaction with client");
                c.clone().send_transaction(&tx.clone(), true).await
            }
        });
        res
    }

    pub async fn submit(&self) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let transaction = self.generator.lock().unwrap().generate_simple_tx()?.clone();
        let res = self.client.clone().send_transaction(&transaction.clone().transaction, true).await?;
        // let res = self.block(self.spawn(transaction.clone().transaction)).await?;
        // if res.clone().accepted() {
        self.generator.lock().unwrap().completed(transaction);
        // }
        // info!("Submit response: {}", res.json_or());
        res.at_least_1()?;
        Ok(res)
    }
    pub async fn submit_test_contract(&self) -> RgResult<SubmitTransactionResponse> {
        let tk = self.generator.lock().unwrap().generate_deploy_test_contract().await?;
        let res = self.client.clone().send_transaction(&tk.transaction, true).await?;
        self.generator.lock().unwrap().completed(tk);
        res.at_least_1()?;
        Ok(res)
    }

    // Direct output contract invocation call.
    pub async fn submit_test_contract_call(&self, contract_address: &Address, contract_utxo: &FixedUtxoId) -> RgResult<SubmitTransactionResponse> {
        let tk = self.generator.lock().unwrap().generate_deploy_test_contract_request(
            contract_address.clone(),
            contract_utxo.clone(),
        ).await?;
        let res = self.client.clone().send_transaction(&tk.transaction, true).await?;
        self.generator.lock().unwrap().completed(tk);
        res.at_least_1()?;
        Ok(res)
    }


    pub async fn drain(&self, to: Address) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let transaction = self.generator.lock().unwrap().drain_tx(&to).clone();
        let res = self.block(self.spawn(transaction.clone())).await;
        res
    }

    pub async fn with_faucet(&self) -> Result<FaucetResponse, ErrorInfo> {
        let pc = &self.client;
        let w = MnemonicWords::from_iterated_phrase("random").key_at(0);
        let a = w.address_typed();
        let _vec_a = a.address.safe_bytes()?;
        let res = //self.runtime.block_on(
            pc.faucet(&a).await?;
        let submit_r = res.submit_transaction_response.safe_get()?;
        let qry = submit_r.query_transaction_response.safe_get()?;
        submit_r.at_least_1()?;
        let len_proofs = qry.observation_proofs.len();
        if len_proofs < 2 {
            return Err(error_info("Insufficient observation proofs"));
        }
        // let h = res.transaction_hash.expect("tx hash");
        // let q = self.runtime.block_on(pc.query_hash(h.hex())).expect("query hash");
        // let tx_info = q.transaction_info.expect("transaction");
        // assert!(!tx_info.observation_proofs.is_empty());
        // let tx = tx_info.transaction.expect("tx");// TODO: does this matter 0 time?
        let tx = submit_r.transaction.as_ref().expect("tx");
        let vec = tx.to_utxo_entries(0);
        let vec1 = vec.iter().filter(|u|
            u.address == Some(a.clone())).collect_vec();
        let matching_entries = vec1.get(0);
        let utxos = matching_entries.expect("utxo").clone().clone();

        self.generator.lock().unwrap().finished_pool.push(
            SpendableUTXO{
                utxo_entry: utxos,
                key_pair: w
            });
        Ok(res)
    }

    // TODO: make interior here a function
    pub async fn submit_split(&self) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let transaction = self.generator.lock().unwrap().generate_split_tx().clone();
        let mut h = vec![];
        for x in transaction {
            h.push((self.spawn(x.clone().transaction), x));
        }
        self.await_results(h).await
    }

    pub async fn submit_duplicate(&self) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let transaction = self.generator.lock().unwrap().generate_simple_tx().clone().expect("tx");
        let h1 = self.spawn(transaction.clone().transaction);
        let h2 = self.spawn(transaction.clone().transaction);
        let dups = self.await_results(vec![(h1, transaction.clone()), (h2, transaction.clone())]).await;
        info!("{}", serde_json::to_string(&dups.clone()).unwrap());
        assert!(dups.iter().any(|x| x.is_ok()));
        assert!(dups.iter().any(|x| !x.is_err()));
        assert!(dups.iter().any(|x|
            x.clone().err().filter(|e| e.code == Error::TransactionAlreadyProcessing as i32).is_some()
        ));
        assert!(dups.iter().any(|x|
            x.as_ref().map(|q| q.at_least_1().is_ok()
            ).unwrap_or(false)
        ));

        dups
    }

    async fn await_results(
        &self,
        handles: Vec<(JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>>, TransactionWithKey)>,
    ) -> Vec<Result<SubmitTransactionResponse, ErrorInfo>> {
        let mut results = vec![];
        for (h, transaction) in handles {
            let res = self.block(h).await;
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

    pub(crate) async fn submit_double_spend(
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
        let doubles = self.await_results(vec![(h1, t1.clone()), (h2, t2.clone())]).await;
        // info!("Double spend test response: {}", serde_json::to_string(&doubles.clone()).unwrap());

        assert!(doubles.iter().any(|x| x.is_ok()));
        assert!(doubles.iter().any(|x|
            x.as_ref().map(|q| q.accepted(1).is_ok()).unwrap_or(false)));
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

    async fn block(&self, jh: JoinHandle<Result<SubmitTransactionResponse, ErrorInfo>>) -> Result<SubmitTransactionResponse, ErrorInfo> {
        jh.await.error_info("submit joinhandle error")?
    }
}
