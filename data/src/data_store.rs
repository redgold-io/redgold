use std::path::{Path, PathBuf};
use std::result;
use std::sync::Arc;

//use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use log::info;
use metrics::gauge;
use sqlx::{Acquire, Sqlite, SqlitePool};
use sqlx::{Row};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

use crate::address_block::AddressBlockStore;
use crate::config::ConfigStore;
use crate::DataStoreContext;
use crate::mp_store::MultipartyStore;
use crate::observation_store::ObservationStore;
use crate::peer::PeerStore;
use crate::transaction_store::TransactionStore;
use redgold_schema::{EasyJson, error_info, ErrorInfoContext, RgResult, SafeOption, structs};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AddressInfo, Hash, Transaction, TransactionInfo, TransactionState, UtxoEntry};

use crate::schema::structs::{
    Address, ErrorInfo,
};
use crate::state_store::StateStore;
use crate::utxo_store::UtxoStore;

#[derive(Clone)]
pub struct DataStore {
    pub connection_path: String,
    pub pool: Arc<SqlitePool>,
    pub address_block_store: AddressBlockStore,
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


    // TODO: Do this as a single sqlite transaction.
    pub async fn accept_transaction(&self,
                                    tx: &Transaction,
                                    time: i64,
                                    rejection_reason: Option<ErrorInfo>,
                                    update_utxo: bool
    ) -> RgResult<()> {

        // let mut pool = self.ctx.pool().await?;
        // let mut sqlite_tx = DataStoreContext::map_err_sqlx(pool.begin().await)?;
        let mut sqlite_tx: Option<&mut sqlx::Transaction<'_, Sqlite>> = None;
        let result = self.accept_transaction_inner(tx, time, rejection_reason, update_utxo, sqlite_tx).await;
        result
        // match result {
        //     Ok(_) => {
        //         sqlite_tx.commit().await.error_info("Sqlite commit failure")?;
        //         Ok(())
        //     }
        //     Err(e) => {
        //         sqlite_tx.rollback().await.error_info("Rollback failure").with_detail("original_error", e.json_or())
        //     }
        // }
    }

    async fn accept_transaction_inner(
        &self,
        tx: &Transaction, time: i64, rejection_reason: Option<ErrorInfo>, update_utxo: bool,
        mut sqlite_tx: Option<&mut sqlx::Transaction<'_, Sqlite>>) -> RgResult<()> {
        self.insert_transaction(
            tx, time, rejection_reason.clone(), update_utxo, None
        ).await?;

        if rejection_reason.is_none() {
            for utxo_id in tx.input_utxo_ids() {
                self.utxo.delete_utxo(utxo_id).await?;
                if self.utxo.utxo_id_valid(utxo_id).await? {
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
        sqlite_tx: Option<&mut sqlx::Transaction<'_, Sqlite>>
    ) -> Result<i64, ErrorInfo> {
        let i = self.transaction_store.insert_transaction_raw(tx, time.clone(), rejection_reason.clone(), sqlite_tx).await?;
        if update_utxo && rejection_reason.is_none() {
            for entry in UtxoEntry::from_transaction(tx, time.clone()) {
                let id = entry.utxo_id.safe_get_msg("malfored utxoid on formation in insert transaction")?;
                if self.utxo.utxo_child(id).await?.is_none() {
                    self.transaction_store.insert_utxo(&entry, None).await?;
                }
            }
        }
        if rejection_reason.is_none() {
            self.transaction_store.insert_transaction_indexes(&tx, time).await?;
            gauge!("redgold.transaction.accepted.total").increment(1.0);
        } else {
            gauge!("redgold.transaction.rejected.total").increment(1.0);
        }

        return Ok(i);
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
            let valid_utxo_output_ids = self.transaction_store
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
        let tx_count = self.transaction_store.count_total_accepted_transactions().await?;
        gauge!("redgold.transaction.accepted.total").set(tx_count as f64);
        let obs_count = self.observation.count_total_observations().await?;
        gauge!("redgold.observation.total").set(obs_count as f64);
        let utxo_total = self.transaction_store.count_total_utxos().await?;
        gauge!("redgold.utxo.total").set(utxo_total as f64);
        Ok(())
    }

    // TODO: Move to utxoStore
    pub async fn get_address_string_info(&self, address: String) -> Result<AddressInfo, ErrorInfo> {
        let addr = Address::parse(address)?;
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

    pub async fn from_path(path: String) -> DataStore {
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
        let ctx = DataStoreContext { connection_path: path.clone(), pool: pl.clone() };
        DataStore {
            ctx: ctx.clone(),
            connection_path: path.clone(),
            pool: pl.clone(),
            address_block_store: AddressBlockStore{ ctx: ctx.clone() },
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
        DataStore::from_path(format!("{}{}", "file:", path.to_str().expect("").to_string())).await
    }

    pub async fn from_file_path(path: String) -> DataStore {
        DataStore::from_path(format!("{}{}", "file:", path)).await
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

