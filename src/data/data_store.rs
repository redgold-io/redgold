use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::result;
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use log::info;
use metrics::{gauge, increment_counter};
use rusqlite::{params, Connection, Error, Result};
use sqlx::migrate::MigrateError;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteRow};
use sqlx::{Row, Sqlite, SqlitePool};
use redgold_data::address_block::AddressBlockStore;
use redgold_data::DataStoreContext;
use redgold_data::peer::PeerStore;
use redgold_data::config::ConfigStore;
use redgold_data::mp_store::MultipartyStore;
use redgold_data::observation_store::ObservationStore;
use redgold_data::servers::ServerStore;
use redgold_data::transaction_store::TransactionStore;

use crate::data::download::DownloadMaxTimes;
use crate::data::utxo;
use crate::genesis::create_genesis_transaction;
use crate::node_config::NodeConfig;
use crate::schema::structs::{
    Address, AddressBlock, Block, ErrorInfo, ObservationEdge, ObservationEntry, ObservationProof,
    TransactionEntry,
};
use crate::schema::structs::{Error as RGError, Hash};
use crate::schema::structs::{MerkleProof, Output};
use crate::schema::structs::{Observation, UtxoEntry};
use crate::schema::structs::{ObservationMetadata, Proof, Transaction};
use crate::schema::TestConstants;
use crate::schema::{ProtoHashable, SafeBytesAccess, WithMetadataHashable};
use crate::util::cli::args::{empty_args, RgArgs};
use crate::util::keys::public_key_from_bytes;
// use crate::util::to_libp2p_peer_id;
use crate::{schema, util};
use crate::schema::structs;
use redgold_schema::constants::EARLIEST_TIME;
use redgold_schema::{error_info, error_message, ProtoSerde, RgResult, SafeOption};
use redgold_schema::structs::{AddressInfo, NetworkEnvironment};
use redgold_schema::transaction::AddressBalance;
use crate::util::cli::arg_parse_config::ArgTranslate;

/*

   "CREATE TABLE IF NOT EXISTS utxos (value INTEGER, keychain TEXT, vout INTEGER, txid BLOB, script BLOB);",
   "CREATE INDEX idx_txid_vout ON utxos(txid, vout);",

   should we remove the utxo id direct key and just use two separate values?
*/
//https://github.com/launchbadge/sqlx should use this instead?
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


#[derive(Clone)]
pub struct RewardQueryResult {
    pub reward_address: Vec<u8>,
    pub deterministic_trust: f64,
}
#[derive(Clone)]
pub struct PeerQueryResult {
    pub public_key: Vec<u8>,
    pub trust: f64,
}
//
// impl PeerQueryResult {
//     pub fn to_peer_id(&self) -> PeerId {
//         to_libp2p_peer_id(&public_key_from_bytes(&self.public_key).unwrap())
//     }
// }

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

    pub fn connection(&self) -> rusqlite::Result<Connection, Error> {
        return Connection::open(self.connection_path.clone());
    }

    pub async fn pool(&self) -> std::result::Result<PoolConnection<Sqlite>, ErrorInfo> {
        DataStore::map_err_sqlx(self.pool.acquire().await)
    }

    // TODO: Move to utxoStore
    pub async fn get_address_string_info(&self, address: String) -> Result<AddressInfo, ErrorInfo> {
        let addr = Address::parse(address)?;
        let res = self.transaction_store.query_utxo_address(&addr).await?;
        Ok(AddressInfo::from_utxo_entries(addr.clone(), res))
    }
    //
    // pub fn select_latest_reward_hash(&self) -> Result<Vec<u8>, Error> {
    //     let conn = self.connection()?;
    //     let mut statement = conn.prepare("SELECT hash FROM rewards ORDER BY time DESC LIMIT 1")?;
    //     let mut rows = statement.query_map(params![], |row| {
    //         let hash: Vec<u8> = row.get(0)?;
    //         Ok(hash)
    //     })?;
    //     Ok(rows.next().unwrap().unwrap())
    // }
    //
    // pub fn select_reward_weights(&self) -> Result<Vec<RewardQueryResult>, Error> {
    //     let conn = self.connection()?;
    //     let mut statement =
    //         conn.prepare("SELECT reward_address, deterministic_trust FROM peers")?;
    //     let rows = statement.query_map(params![], |row| {
    //         let result = RewardQueryResult {
    //             reward_address: row.get(0)?,
    //             deterministic_trust: row.get(1)?,
    //         };
    //         Ok(result)
    //     })?;
    //     Ok(rows
    //         .filter(|x| x.is_ok())
    //         .map(|x| x.unwrap())
    //         .collect::<Vec<RewardQueryResult>>())
    // }
    // pub fn select_peer_trust(
    //     &self,
    //     peer_ids: &Vec<Vec<u8>>,
    // ) -> Result<HashMap<Vec<u8>, f64>, Error> {
    //     let conn = self.connection()?;
    //     let mut map: HashMap<Vec<u8>, f64> = HashMap::new();
    //
    //     for peer_id in peer_ids {
    //         let mut statement = conn.prepare("SELECT id, trust FROM peers WHERE id = ?1")?;
    //         //.raw_bind_parameter();
    //         // Try this ^ for iterating over peer ids.
    //         let rows = statement.query_map(params![peer_id], |row| {
    //             Ok(PeerTrustQueryResult {
    //                 peer_id: row.get(0)?,
    //                 trust: row.get(1)?,
    //             })
    //         })?;
    //         // // TODO error handling for multiple rows
    //
    //         for row in rows {
    //             let row_q = row?;
    //             map.insert(row_q.peer_id, row_q.trust);
    //         }
    //     }
    //     return Ok(map);
    // }


    pub fn map_err<A>(error: rusqlite::Result<A, rusqlite::Error>) -> result::Result<A, ErrorInfo> {
        error.map_err(|e| error_info(e.to_string()))
    }

    pub fn map_err_sqlx<A>(error: result::Result<A, sqlx::Error>) -> result::Result<A, ErrorInfo> {
        error.map_err(|e| error_message(schema::structs::Error::InternalDatabaseError, e.to_string()))
    }


    pub fn select_all_tables(&self) -> rusqlite::Result<Vec<String>, Error> {
        let conn = self.connection()?;

        let mut statement = conn.prepare(
            "SELECT
    name
FROM
    sqlite_master
WHERE
    type ='table' AND
    name NOT LIKE 'sqlite_%';",
        )?;

        let rows = statement.query_map([], |row| {
            let str: String = row.get(0)?;
            Ok(str)
        })?;
        return Ok(rows.map(|r| r.unwrap()).collect_vec());
    }
    //
    // pub fn get_max_time_old(&self, table: &str) -> rusqlite::Result<i64, Error> {
    //     let conn = self.connection()?;
    //     let query = "SELECT max(time) FROM ".to_owned() + table;
    //     let mut statement = conn.prepare(&*query)?;
    //     let mut rows = statement.query_map(params![], |r| {
    //         let data: i64 = r.get(0)?;
    //         Ok(data)
    //     })?;
    //     let row = rows.next().unwrap().unwrap_or(0 as i64);
    //     Ok(row)
    // }
    //
    // pub async fn get_max_time(&self, table: &str) -> RgResult<i64> {
    //     let mut pool = self.ctx.pool().await?;
    //     let query = "SELECT max(time) as max_time FROM ".to_owned() + table;
    //     let mut query = sqlx::query(&query);
    //     let rows = query.fetch_all(&mut pool).await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     for row in rows_m {
    //         let raw: i64 = DataStoreContext::map_err_sqlx(row.try_get("max_time"))?;
    //         return Ok(raw)
    //     }
    //     return Err(error_info("No max time found"))
    // }
    //
    // pub fn query_download_times(&self) -> DownloadMaxTimes {
    //     DownloadMaxTimes {
    //         utxo: self.get_max_time("utxo").await.unwrap(),
    //         transaction: self.get_max_time("transactions").await.unwrap(),
    //         observation: self.get_max_time("observation").await.unwrap(),
    //         observation_edge: self.get_max_time("observation_edge").await.unwrap(),
    //     }
    // }


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


    pub async fn from_config(node_config: &NodeConfig) -> DataStore {
        DataStore::from_path(format!("{}{}", "file:", node_config.env_data_folder().data_store_path().to_str().expect("").to_string())).await
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
            tracing::error!("Migration related failure, attempting to handle");
            if e.message.contains("was previously applied") && allow_delete {
                tracing::error!("Found prior conflicting schema -- but ok to remove; removing existing datastore and exiting");
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


#[allow(dead_code)]
#[derive(sqlx::FromRow)]
pub struct MnemonicEntry {
    pub(crate) words: String,
    pub(crate) time: i64,
    pub(crate) peer_id: Vec<u8>,
}

