use std::path::{Path, PathBuf};
use std::result;
use std::sync::Arc;
use futures::future::Either;
use futures::{Stream, StreamExt};

//use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use log::info;
use metrics::{counter, gauge};
use serde_json::de::Read;
use sqlx::{Acquire, Sqlite, SqlitePool};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use redgold_keys::address_support::AddressSupport;

use crate::config::ConfigStore;
use crate::DataStoreContext;
use crate::mp_store::MultipartyStore;
use crate::observation_store::ObservationStore;
use crate::peer::PeerStore;
use crate::transaction_store::TransactionStore;
use redgold_schema::{EasyJson, error_info, ErrorInfoContext, RgResult, SafeOption, structs, util};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{AddressInfo, Hash, Transaction, TransactionInfo, TransactionState, UtxoEntry, UtxoId};
use redgold_schema::util::machine_info::{available_bytes, cores_total, file_size_bytes, memory_total_kb};

use crate::schema::structs::{
    Address, ErrorInfo,
};
use crate::state_store::StateStore;
use crate::utxo_store::UtxoStore;

#[derive(Clone)]
pub struct DataStore {
    pub connection_path: String,
    pub pool: Arc<SqlitePool>,
    pub peer_store: PeerStore,
    pub config_store: ConfigStore,
    pub transaction_store: TransactionStore,
    pub multiparty_store: MultipartyStore,
    //pub server_store: ServerStore
    pub observation: ObservationStore,
    pub ctx: DataStoreContext,
    pub state: StateStore,
    pub utxo: UtxoStore,
}

impl DataStore {

    //
    // pub async fn parquet_export_archive_historical(&self, path: &str) -> RgResult<()> {
    //     let mut obs_stream = self.observation.accepted_time_observation_ordered(0, util::current_time_millis()).await?;
    //     let mut transaction_stream = self.transaction_store
    //         .transaction_accepted_ordered_stream(0, util::current_time_millis()).await?;
    //
    //     let mut results = vec![];
    //
    //     let mut latest_tx = transaction_stream.next().await;
    //     let mut latest_obs = obs_stream.next().await;
    //     let mut is_tx_next = true;
    //
    //     let mut remainder: Option<(Transaction, i64)> = {
    //         // TODO: if let Some
    //         let t = latest_tx.safe_get_msg("No latest transaction")?.clone()?;
    //         let o = latest_obs.safe_get_msg("No latest observation")?.clone()?;
    //         let tt = t.time()?.clone();
    //         let ot = o.time()?;
    //         if tt < ot {
    //             is_tx_next = true;
    //             // Observation is remainder due to higher time.
    //             Some((o, tt))
    //         } else {
    //             Some((t, tt))
    //         }
    //     };
    //
    //     loop {
    //         if latest_tx.is_none() && latest_obs.is_none() {
    //             break;
    //         }
    //         let tx = if is_tx_next {
    //             transaction_stream.next().await
    //         } else {
    //             obs_stream.next().await
    //         };
    //
    //         if let Some(tx) = tx {
    //             let tx = tx?;
    //             let time = tx.time()?.clone();
    //             if let Some((r, rt)) = remainder.as_ref() {
    //                 if rt < time {
    //                     results.push(r);
    //                     remainder = (tx, time);
    //                     // Switch streams
    //                     is_tx_next = !is_tx_next;
    //                 }
    //             }
    //         } else {
    //             is_tx_next = !is_tx_next;
    //         }
    //     }
    //
    //     Ok(())
    // }

    // TODO: Do this as a single sqlite transaction.
                                pub async fn accept_transaction(&self,
                                    tx: &Transaction,
                                    time: i64,
                                    rejection_reason: Option<ErrorInfo>,
                                    update_utxo: bool
    ) -> RgResult<()> {

        counter!("redgold_transaction_accept_called").increment(1);

        let mut pool = self.ctx.pool().await?;
        let mut sqlite_tx = DataStoreContext::map_err_sqlx(pool.begin().await)?;
        let result = self
            .accept_transaction_inner(tx, time, rejection_reason.clone(), update_utxo, &mut sqlite_tx).await
            .with_detail_fn("transaction", || tx.json_or())
            .with_detail("rejection_reason", rejection_reason.json_or())
            .with_detail("time", time.to_string())
            .with_detail("update_utxo", update_utxo.to_string());
        // result
        let final_result = match result {
            Ok(_) => {
                sqlite_tx.commit().await.error_info("Sqlite commit failure")?;
                Ok(())
            }
            Err(e) => {
                match sqlite_tx.rollback().await.error_info("Rollback failure").with_detail("original_error", e.json_or()) {
                    Ok(_) => { Err(e)}
                    Err(e) => {Err(e)}
                }
            }
        };
        final_result
    }

    async fn accept_transaction_inner(
        &self,
        tx: &Transaction, time: i64, rejection_reason: Option<ErrorInfo>, update_utxo: bool,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> RgResult<()> {
        let insert_result = self.insert_transaction(
            tx, time, rejection_reason.clone(), update_utxo, sqlite_tx
        ).await;

        insert_result?;

        if rejection_reason.is_none() {
            for utxo_id in tx.input_utxo_ids() {
                self.utxo.delete_utxo(utxo_id, Some(sqlite_tx)).await?;
                if self.utxo.utxo_id_valid_opt(utxo_id, Some(sqlite_tx)).await? {
                    return Err(error_info("UTXO not deleted"));
                }
            }
        }
        Ok(())
    }


    pub async fn insert_transaction(
        &self,
        tx: &Transaction,
        time: i64,
        rejection_reason: Option<ErrorInfo>,
        update_utxo: bool,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<i64, ErrorInfo> {
        let i = self.transaction_store.insert_transaction_raw(tx, time.clone(), rejection_reason.clone(), sqlite_tx).await?;
        if update_utxo && rejection_reason.is_none() {
            for entry in UtxoEntry::from_transaction(tx, time.clone()) {
                let id = entry.utxo_id.safe_get_msg("malformed utxo_id on formation in insert transaction")?;
                if self.utxo.utxo_children_pool_opt(id, Some(sqlite_tx)).await?.len() == 0 {
                    self.utxo.insert_utxo(&entry, sqlite_tx).await?;
                } else {
                    counter!("redgold_utxo_insertion_duplicate").increment(1);
                }
            }
        }
        if rejection_reason.is_none() {
            self.insert_transaction_indexes(&tx, time, sqlite_tx).await?;
            gauge!("redgold_transaction_accepted_total").increment(1.0);
        } else {
            gauge!("redgold.transaction.rejected.total").increment(1.0);
        }

        return Ok(i);
    }

    pub async fn insert_transaction_indexes(
        &self,
        tx: &Transaction,
        time: i64,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<(), ErrorInfo> {
        let hash = &tx.hash_or();
        for (i, input) in tx.inputs.iter().enumerate() {
            if let Some(utxo) = &input.utxo_id {
                self.insert_transaction_edge(
                    utxo,
                    &input.address()?,
                    hash,
                    i as i64,
                    time,
                    sqlite_tx
                ).await?;
                // Ensure UTXO deleted
                self.utxo.delete_utxo(utxo, Some(sqlite_tx)).await?;
                if self.utxo.utxo_id_valid_opt(utxo, Some(sqlite_tx)).await? {
                    return Err(error_info("UTXO not deleted"));
                }
                if self.utxo.utxo_children_pool_opt(utxo, Some(sqlite_tx)).await?.len() == 0 {
                    return Err(error_info("UTXO has no children after insert"));
                }

            }
        }
        self.transaction_store.insert_address_transaction(tx, sqlite_tx).await?;
        Ok(())
    }


    pub async fn insert_transaction_edge(
        &self,
        utxo_id: &UtxoId,
        address: &Address,
        child_transaction_hash: &Hash,
        child_input_index: i64,
        time: i64,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<i64, ErrorInfo> {

        let hash = utxo_id.transaction_hash.safe_get_msg("No transaction hash on utxo_id")?.vec();
        let child_hash = child_transaction_hash.vec();
        let output_index = utxo_id.output_index;
        let address = address.proto_serialize();
        let rows = DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"
        INSERT OR REPLACE INTO transaction_edge
        (transaction_hash, output_index, address, child_transaction_hash, child_input_index, time)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            hash,
            output_index,
            address,
            child_hash,
            child_input_index,
            time
        )
            .execute(&mut **sqlite_tx)
            .await)?;
        Ok(rows.last_insert_rowid())
    }

    pub async fn table_size(&self, table_name: impl Into<String>) -> RgResult<Vec<i64>> {
        let string = table_name.into();
        let string1 = format!(r#"SELECT sum("pgsize") as aggregate FROM dbstat WHERE name='{}'"#, string.as_str());
        let rows = DataStoreContext::map_err_sqlx(sqlx::query(
            // r#"select aggregate from dbstat('main',1) where name='utxo'"#,
            string1.as_str(),

        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?;
        let mut res = vec![];
        for r in rows {
            // let name: String = DataStoreContext::map_err_sqlx(r.try_get("name"))?;
            let aggregate: i64 = DataStoreContext::map_err_sqlx(r.try_get("aggregate"))?;
            res.push(aggregate);
        }
        Ok(res)
    }


    pub async fn table_sizes(&self) -> RgResult<Vec<(String,i64)>> {
        let rows = DataStoreContext::map_err_sqlx(sqlx::query(
            r#"SELECT name, sum("pgsize") as aggregate FROM dbstat GROUP BY name"#
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?;
        let mut res = vec![];
        for r in rows {
            let name: String = DataStoreContext::map_err_sqlx(r.try_get("name"))?;
            let aggregate: i64 = DataStoreContext::map_err_sqlx(r.try_get("aggregate"))?;
            res.push((name, aggregate));
        }
        Ok(res)
    }


    pub async fn resolve_code(&self, address: &Address) -> RgResult<structs::ResolveCodeResponse> {
        let res = self.utxo.code_utxo(address, true).await?;
        let mut resp = structs::ResolveCodeResponse::default();
        resp.utxo_entry = res.clone();

        if let Some(u) = res.as_ref()
            .and_then(|r| r.utxo_id.as_ref())
            .and_then(|r| r.transaction_hash.as_ref()) {
            let tx_res = self.resolve_transaction_hash(u).await?;
            resp.transaction = tx_res;
            resp.contract_state_marker = self.state.query_recent_state(
                address, None, Some(1)).await?.get(0).cloned();
        }
        Ok(resp)
    }

    pub async fn resolve_transaction_hash(&self, hash: &Hash) -> RgResult<Option<TransactionInfo>> {
        let maybe_transaction = self.transaction_store.query_maybe_transaction(&hash).await?;
        let mut observation_proofs = vec![];
        let mut transaction = None;
        let mut rejection_reason = None;
        let mut state = TransactionState::ObservedPending;
        let mut transaction_info: Option<TransactionInfo> = None;

        if let Some((t, e)) = maybe_transaction.clone() {
            observation_proofs = self.observation.select_observation_edge(&hash.clone()).await?;
            transaction = Some(t);
            rejection_reason = e;
            // Query UTXO by hash only for all valid outputs.
            let valid_utxo_output_ids = self.utxo
                .query_utxo_output_index(&hash)
                .await?;

            let accepted = rejection_reason.is_none();

            if accepted {
                state = TransactionState::ObservedAccepted;
            } else {
                state = TransactionState::Rejected
            }

            let mut tx_info = TransactionInfo::default();
            tx_info.transaction = transaction.clone();
            tx_info.observation_proofs = observation_proofs.clone();
            tx_info.valid_utxo_index = valid_utxo_output_ids.clone();
            tx_info.used_outputs = vec![];
            tx_info.accepted = accepted;
            tx_info.rejection_reason = rejection_reason.clone();
            tx_info.queried_output_index_valid = None;
            tx_info.state = state as i32;
            transaction_info = Some(tx_info);
        }
        Ok(transaction_info)
    }

}

impl DataStore {

    // Example fix for schema migration
    pub async fn check_consistency_apply_fixes(&self) -> RgResult<()> {
        // let lim = 10000;
        // let mut offset = 0;
        // while {
        //     let res = self.transaction_store.utxo_scroll(lim, 0).await?;
        //     for r in res {
        //
        //     }
        //     res.len() == lim
        // } {}
        Ok(())
    }
}

impl DataStore {

    pub async fn count_gauges(&self) -> RgResult<()> {
        gauge!("redgold_transaction_accepted_total").set(self.transaction_store.count_total_accepted_transactions().await? as f64);
        gauge!("redgold_observation_total").set(self.observation.count_total_observations().await? as f64);
        gauge!("redgold_utxo_total").set(self.transaction_store.count_total_utxos().await? as f64);
        gauge!("redgold_utxo_distinct_addresses").set(self.utxo.count_distinct_address_utxo().await? as f64);
        gauge!("redgold_disk_available_gigabytes").set(available_bytes(self.ctx.file_path.clone(), false).log_error().unwrap_or(0) as f64 / (1024f64*1024f64*1024f64));
        gauge!("redgold_data_store_size_gigabytes").set(file_size_bytes(self.ctx.file_path.clone()).log_error().unwrap_or(0) as f64  / (1024f64*1024f64*1024f64));
        gauge!("redgold_memory_total").set(memory_total_kb().unwrap_or(0) as f64  / (1024f64*1024f64));
        gauge!("redgold_cores_total").set(cores_total().log_error().unwrap_or(0) as f64);
        gauge!("redgold_multiparty_total").set(self.multiparty_store.count_multiparty_total().await? as f64);
        gauge!("redgold_multiparty_self").set(self.multiparty_store.count_self_multiparty().await? as f64);
        // self.multiparty_store
        Ok(())
    }

    // TODO: Move to utxoStore
    pub async fn get_address_string_info(&self, address: String) -> Result<AddressInfo, ErrorInfo> {
        let addr = address.parse_address()?;
        let res = self.transaction_store.query_utxo_address(&addr).await?;
        Ok(AddressInfo::from_utxo_entries(addr.clone(), res))
    }


    /*
        pub fn select_all_tables(&self) -> rusqlite::Result<Vec<String>, Error> {

                "SELECT
    name
FROM
    sqlite_master
WHERE
    type ='table' AND
    name NOT LIKE 'sqlite_%';",
     */

    pub async fn from_path(path: String, original_path: String) -> DataStore {
        // info!("Starting datastore with path {}", path.clone());

        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal) // Set journal mode to WAL
            .busy_timeout(std::time::Duration::from_secs(60)) // Set busy timeout to 10 seconds
            .filename(Path::new(&path.clone()));
        /*

         */
        let pool = SqlitePool::connect_with(options)
            .await
            .expect("Connection failure");
        // info!("Opened pool");
        let pl = Arc::new(pool);
        let ctx = DataStoreContext { file_path: original_path.clone(), connection_path: path.clone(), pool: pl.clone() };
        DataStore {
            ctx: ctx.clone(),
            connection_path: path.clone(),
            pool: pl.clone(),
            peer_store: PeerStore{ ctx: ctx.clone() },
            config_store: ConfigStore{ ctx: ctx.clone() },
            // server_store: ServerStore{ ctx: ctx.clone() },
            transaction_store: TransactionStore{ ctx: ctx.clone() },
            utxo: UtxoStore{ ctx: ctx.clone() },
            multiparty_store: MultipartyStore { ctx: ctx.clone() },
            observation: ObservationStore { ctx: ctx.clone() },
            state: StateStore { ctx: ctx.clone() },
        }
    }

    // node_config.env_data_folder().data_store_path()
    pub async fn from_config_path(path: &PathBuf) -> DataStore {
        let original_path = path.to_str().expect("").to_string();
        DataStore::from_path(format!("{}{}", "file:", original_path.clone()), original_path).await
    }

    pub async fn from_file_path(path: String) -> DataStore {
        DataStore::from_path(format!("{}{}", "file:", path.clone()), path).await
    }



    pub async fn run_migrations(&self) -> result::Result<(), ErrorInfo> {
        self.ctx.run_migrations().await

    }

    pub async fn run_migrations_fallback_delete(&self, allow_delete: bool, data_store_file_path: PathBuf) -> result::Result<(), ErrorInfo> {
        let migration_result = self.run_migrations().await;
        if let Err(e) = migration_result {
            log::error!("Migration related failure, attempting to handle");
            if e.message.contains("was previously applied") && allow_delete {
                log::error!("Found prior conflicting schema -- but ok to remove; removing existing datastore and exiting");
                std::fs::remove_file(data_store_file_path.clone())
                    .map_err(|e| error_info(format!("Couldn't remove existing datastore: {}", e.to_string())))?;
                // panic!("Exiting due to ds removal, should work on retry");
            } // else {
            return Err(e);
            //}
        }
        Ok(())
    }

}

