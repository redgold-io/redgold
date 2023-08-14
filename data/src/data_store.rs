use std::path::{Path, PathBuf};
use std::result;
use std::sync::Arc;

//use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use log::info;
use metrics::gauge;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

use crate::address_block::AddressBlockStore;
use crate::config::ConfigStore;
use crate::DataStoreContext;
use crate::mp_store::MultipartyStore;
use crate::observation_store::ObservationStore;
use crate::peer::PeerStore;
use crate::transaction_store::TransactionStore;
use redgold_schema::{error_info, RgResult};
use redgold_schema::structs::AddressInfo;

use crate::schema::structs::{
    Address, ErrorInfo,
};

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
        gauge!("redgold.transaction.accepted.total", tx_count as f64);
        let obs_count = self.observation.count_total_observations().await?;
        gauge!("redgold.observation.total", obs_count as f64);
        let utxo_total = self.transaction_store.count_total_utxos().await?;
        gauge!("redgold.utxo.total", utxo_total as f64);
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
        info!("Starting datastore with path {}", path.clone());

        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(Path::new(&path.clone()));
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
            multiparty_store: MultipartyStore { ctx: ctx.clone() },
            observation: ObservationStore { ctx: ctx.clone() },
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

