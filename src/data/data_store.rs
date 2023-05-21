use std::collections::HashMap;
use std::path::Path;
use std::result;
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use libp2p::PeerId;
use log::info;
use metrics::increment_counter;
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
use crate::util::cli::args::empty_args;
use crate::util::keys::public_key_from_bytes;
use crate::util::to_libp2p_peer_id;
use crate::{schema, util};
use crate::schema::structs;
use redgold_schema::constants::EARLIEST_TIME;
use redgold_schema::{error_info, error_message, ProtoSerde, SafeOption};
use redgold_schema::structs::AddressInfo;
use redgold_schema::transaction::AddressBalance;

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
}

/*

   let pool = SqlitePool::connect(&*path.clone())
       .await
       .expect("Connection failure");
*/
#[derive(Clone)]
pub struct PeerTrustQueryResult {
    peer_id: Vec<u8>,
    trust: f64,
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

impl PeerQueryResult {
    pub fn to_peer_id(&self) -> PeerId {
        to_libp2p_peer_id(&public_key_from_bytes(&self.public_key).unwrap())
    }
}

impl DataStore {
    pub fn connection(&self) -> rusqlite::Result<Connection, Error> {
        return Connection::open(self.connection_path.clone());
    }

    pub fn create_transactions(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS transactions (
                  hash    BLOB PRIMARY KEY,
                  raw_transaction       BLOB,
                  time       INTEGER
                  )",
                [],
            )
        });
    }

    // Change utxo schema? What fields are we actually going to query here?
    pub fn create_utxo(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS utxo (
                  transaction_hash BLOB,
                  output_index INTEGER,
                  address    BLOB,
                  output    BLOB,
                  time INTEGER,
                  PRIMARY KEY (transaction_hash, output_index)
                  )",
                [],
            )
        });
    }

    // Change utxo schema? What fields are we actually going to query here?
    pub fn create_debug_table(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS debug_table (
                  something STRING PRIMARY KEY,
                  output_index INTEGER
                  )",
                [],
            )
        });
    }

    fn create_peer_keys(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS peer_key (
                  public_key    BLOB PRIMARY KEY,
                  id    BLOB,
                  multi_hash    BLOB,
                  address    TEXT,
                  status    TEXT
                  )",
                [],
            )
        });
    }

    fn create_observation(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS observation (
                  root    BLOB PRIMARY KEY,
                  raw_observation    BLOB,
                  public_key    BLOB,
                  proof    BLOB,
                  time INTEGER
                  )",
                [],
            )
        });
    }

    fn create_observation_edge(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS observation_edge (
                  root    BLOB NOT NULL,
                  leaf_hash    BLOB NOT NULL,
                  observation_hash    BLOB NOT NULL,
                  observation_metadata BLOB NOT NULL,
                  merkle_proof BLOB NOT NULL,
                  time INTEGER,
                  PRIMARY KEY(observation_hash, leaf_hash, root)
                  )",
                [],
            )
        });
    }
    //
    // fn create_mnemonic(&self) -> rusqlite::Result<usize, Error> {
    //     return self.connection().and_then(|c| {
    //         c.execute(
    //             "CREATE TABLE IF NOT EXISTS mnemonic (
    //               words    STRING PRIMARY KEY,
    //               time INTEGER,
    //               peer_id BLOB
    //               )",
    //             /*
    //               ,
    //             rounds INTEGER,
    //             iv BLOB,
    //             password_id BLOB
    //                */
    //             [],
    //         )
    //     });
    // }

    #[allow(dead_code)]
    pub(crate) async fn create_mnemonic(&self) -> result::Result<usize, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let tbl = "CREATE TABLE IF NOT EXISTS mnemonic (
                  words    STRING PRIMARY KEY,
                  time INTEGER,
                  peer_id BLOB
                  )";
        // TODO: Read from resource directory as included byte string
        sqlx::query(tbl).fetch_all(&mut conn).await?;
        Ok(0)
    }

    pub fn map_sqlx_error(err: sqlx::Error) -> ErrorInfo {
        error_info(err.to_string()) // TODO: code
    }

    pub async fn insert_mnemonic(
        &self,
        mnemonic_entry: MnemonicEntry,
    ) -> result::Result<(), sqlx::Error> {
        let mut conn = self.pool.acquire().await?;

        sqlx::query("INSERT INTO mnemonic (words, time, peer_id) VALUES (?, ?, ?)")
            .bind(mnemonic_entry.words)
            .bind(mnemonic_entry.time)
            .bind(mnemonic_entry.peer_id)
            .fetch_all(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn query_all_mnemonic(&self) -> result::Result<Vec<MnemonicEntry>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;

        let rows = sqlx::query("select words, time, peer_id from mnemonic")
            .map(|x: SqliteRow| {
                let words: &str = x.try_get("words")?;
                let time: i64 = x.try_get("time")?;
                let peer_id: Vec<u8> = x.try_get("peer_id")?;
                let res: Result<MnemonicEntry, sqlx::Error> = Ok(MnemonicEntry {
                    words: words.to_string(),
                    time,
                    peer_id,
                });
                res
            })
            .fetch_all(&mut conn)
            .await?;

        let mut res: Vec<MnemonicEntry> = vec![];
        for row in rows {
            let result: MnemonicEntry = row?;
            res.push(result);
        }
        Ok(res)
    }

    pub async fn pool(&self) -> std::result::Result<PoolConnection<Sqlite>, ErrorInfo> {
        DataStore::map_err_sqlx(self.pool.acquire().await)
    }

    pub async fn query_last_block(&self) -> result::Result<Option<Block>, ErrorInfo> {
        let mut pool = self.pool().await?;

        // TODO change this to a fetch all in case nothing is returned on initialization.
        let rows = sqlx::query!("SELECT raw FROM block ORDER BY height DESC LIMIT 1")
            .fetch_one(&mut pool)
            .await;
        let rows_m = DataStore::map_err_sqlx(rows)?;
        match rows_m.raw {
            None => Ok(None),
            Some(b) => Ok(Some(Block::proto_deserialize(b)?)),
        }
    }

    pub async fn query_last_balance_address(
        &self,
        address: &Vec<u8>,
    ) -> result::Result<Option<i64>, ErrorInfo> {
        let mut pool = self.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT balance FROM address_block WHERE address = ?1 ORDER BY height DESC LIMIT 1"#,
            address
        )
        .fetch_all(&mut pool)
        .await;
        let rows_m = DataStore::map_err_sqlx(rows)?;
        for row in rows_m {
            return Ok(row.balance)
        }
        Ok(None)
    }

    pub async fn insert_address_block(
        &self,
        address_block: AddressBlock,
    ) -> result::Result<i64, ErrorInfo> {
        let mut pool = self.pool().await?;

        let rows = sqlx::query!(
            r#"
        INSERT INTO address_block ( address, balance, height, hash )
        VALUES ( ?1, ?2, ?3, ?4)
                "#,
            address_block.address,
            address_block.balance,
            address_block.height,
            address_block.hash
        )
        .execute(&mut pool)
        .await;
        let rows_m = DataStore::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn query_address_balance_by_height(
        &self,
        address: Address,
        height: i64,
    ) -> result::Result<Option<i64>, ErrorInfo> {
        // select address balance
        let address_bytes = address.address.safe_bytes()?;
        let mut pool = self.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT balance FROM address_block WHERE address = ?1 AND height <= ?2 ORDER BY height DESC LIMIT 1"#,
            address_bytes,
            height
        )
        .fetch_all(&mut pool)
        .await;
        let rows_m = DataStore::map_err_sqlx(rows)?;
        for row in rows_m {
            return Ok(row.balance);
        }
        Ok(None)
    }

    // TODO: Just resolve the hash to a height, way easier.
    pub async fn query_block_hash_height(
        &self,
        hash: Option<Vec<u8>>,
        height: Option<i64>,
    ) -> result::Result<Option<Block>, ErrorInfo> {
        if hash.is_none() && height.is_none() {
            return Err(error_message(
                RGError::MissingField,
                "Block hash and height both empty",
            ));
        }
        let mut pool = self.pool().await?;
        let mut clause_str: String = "WHERE ".to_string();
        if hash.is_some() {
            clause_str += "hash = ?1";
            if height.is_some() {
                clause_str += " AND height = ?2";
            }
        } else {
            clause_str += "height = ?1";
        }

        let query_str = format!("SELECT raw FROM block {} LIMIT 2", clause_str);
        let mut query = sqlx::query(&query_str);
        if let Some(h) = hash.clone() {
            query = query.bind(h);
            if let Some(hh) = height {
                query = query.bind(hh);
            }
        } else {
            query = query.bind(height.expect("Height shouldn't be empty with earlier validator"));
        }

        let rows = query.fetch_all(&mut pool).await;

        let rows_m: Vec<_> = DataStore::map_err_sqlx(rows)?;
        if rows_m.is_empty() {
            return Ok(None);
        }
        if rows_m.len() > 1 {
            return Err(error_message(
                RGError::DataStoreInternalCorruption,
                format!(
                    "More than 1 block returned on query for hash {} height {}",
                    hash.map(|h| hex::encode(h)).unwrap_or("none".to_string()),
                    height.map(|h| h.to_string()).unwrap_or("none".to_string())
                ),
            ));
        }
        for row in rows_m {
            let raw: Vec<u8> = DataStore::map_err_sqlx(row.try_get("raw"))?;
            return Ok(Some(Block::proto_deserialize(raw)?));
        }
        return Ok(None);
    }


    pub async fn insert_block_update_historicals(&self, block: &Block) -> Result<(), ErrorInfo> {
        let vec = block.transactions.clone();
        let height = block.height.clone();
        let block_hash = block.hash_bytes()?;


        let mut deltas: HashMap<Vec<u8>, i64> = HashMap::new();

        for tx in vec {
            for o in tx.outputs {
                for a in &o.address.clone() {
                    for d in &o.data {
                        for amount in d.amount {
                            let address_bytes = a.address.safe_bytes()?;
                            let a1 = address_bytes.clone();
                            let maybe_amount = deltas.get(&a1);
                            deltas.insert(
                                address_bytes,
                                maybe_amount.map(|m| m.clone() + amount).unwrap_or(amount),
                            );
                        }
                    }
                }
            }
            for i in tx.inputs {
                // TODO: There's a better way to persist these than querying transaction
                let tx_hash = i.transaction_hash.safe_bytes()?;
                let tx_input =
                    DataStore::map_err(self.query_transaction(&tx_hash))?
                        .expect("change later");
                let prev_output: Output = tx_input
                    .outputs
                    .get(i.output_index as usize)
                    .expect("change later")
                    .clone();
                for a in prev_output.clone().address {
                    for d in &prev_output.data {
                        for amount in d.amount {
                            let address_bytes = a.address.safe_bytes()?;
                            let a1 = address_bytes.clone();
                            let maybe_amount = deltas.get(&a1);
                            deltas.insert(
                                address_bytes,
                                maybe_amount
                                    .map(|m| m.clone() - amount)
                                    .unwrap_or(-1 * amount),
                            );
                        }
                    }
                }
            }
        }

        for (k, v) in deltas.iter() {
            let vec1 = k.clone();
            let res = self.query_last_balance_address(&vec1).await?;
            let new_balance = match res {
                None => v.clone(),
                Some(rr) => v + rr,
            };
            self.insert_address_block(AddressBlock {
                    address: k.clone(),
                    balance: new_balance,
                    height,
                    hash: block_hash.clone(),
                })
                .await?;
        }
        self.insert_block(&block).await?;
        Ok(())
    }

    pub async fn insert_block(&self, block: &Block) -> Result<i64, ErrorInfo> {
        let mut pool = self.pool().await?;

        let hash = block.hash_bytes()?;
        let height = block.height as i64;
        let raw = block.proto_serialize();
        let time = block.time()? as i64;

        let rows = sqlx::query!(
            r#"
        INSERT INTO block ( hash, height, raw, time )
        VALUES ( ?1, ?2, ?3, ?4)
                "#,
            hash,
            height,
            raw,
            time
        )
        .execute(&mut pool)
        .await;
        let rows_m = DataStore::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    // TODO: Move to utxoStore
    pub async fn get_address_string_info(&self, address: String) -> Result<AddressInfo, ErrorInfo> {
        let addr = Address::parse(address)?;
        let result = self.query_utxo_address(vec![addr.clone()]).await;
        let res = Self::map_err_sqlx(result)?;
        Ok(AddressInfo::from_utxo_entries(addr.clone(), res))
    }

    pub async fn query_utxo_address(
        &self,
        addresses: Vec<Address>,
    ) -> result::Result<Vec<UtxoEntry>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;

        let mut res: Vec<UtxoEntry> = vec![];
        // let vec1 = addresses.iter().map(|a| a.address).collect_vec();
        for address in addresses {
            // TODO:
            // If this sql query has a syntax error, it breaks the canary but NOT the e2e? wtf
            let rows = sqlx::query(
                "SELECT transaction_hash, output_index, address, output, time FROM utxo WHERE address = ?"
            ).bind(address.address.clone().safe_bytes().expect("bytes"))
                .map(|row: SqliteRow| {
                    let transaction_hash = row.try_get(0)?;
                    let output_index = row.try_get(1)?;
                    let address = row.try_get(2)?;
                    let time: i64 = row.try_get(4)?;
                    let output = Output::proto_deserialize(row.try_get(3)?).unwrap();
                    let res: Result<UtxoEntry, sqlx::Error> = Ok(UtxoEntry {
                        transaction_hash,
                        output_index,
                        address,
                        output: Some(output),
                        time, // TODO: lol
                    });
                    res
                })
                .fetch_all(&mut conn)
                .await?;
            for row in rows {
                let result: UtxoEntry = row?;
                res.push(result);
            }
        }
        Ok(res)
    }

    /*


    pub fn query_utxo_all_debug(&self) -> rusqlite::Result<Vec<UtxoEntry>, Error> {
        let conn = self.connection()?;
        let mut statement =
            conn.prepare("SELECT transaction_hash, output_index, address, output, time FROM utxo")?;

        let rows = statement.query_map(params![], |row| {
            Ok(UtxoEntry {
                transaction_hash: row.get(0)?,
                output_index: row.get(1)?,
                address: row.get(2)?,
                output: Output::proto_deserialize(row.get(3)?).unwrap(),
                time: row.get(4)?,
            })
        })?;
        // TODO error handling for multiple rows
        let unwrapped = rows.map(|r| r.unwrap());
        return Ok(unwrapped.collect_vec());
    }

     */

    /*
    use sqlx::Connection;
    // DATABASE_URL=sqlite:///Users//test_sqlite.sqlite
    let path = DataStore::in_memory().connection_path;
    //std::env::set_var("DATABASE_URL", path.clone());


    let pool = SqlitePool::connect(&*path.clone())
        .await
        .expect("Connection failure");
    let mut conn = pool.acquire().await.expect("acquire failure");
    // conn.
    let res = sqlx::query(tbl)
        .fetch_all(&mut conn)
        .await
        .expect("create failure");

    let res3 = sqlx::query("select words, time, peer_id from mnemonic")
        .fetch_all(&mut conn)
        .await
        .expect("create failure");
    let res4 = res3.get(0).expect("something");

    let wordss: &str = res4.try_get("words").expect("yes");
    println!("{:?}", wordss);
    assert_eq!(wordss "yo");
     */

    #[allow(dead_code)]
    fn create_private_key(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS private_key (
                  data    BLOB PRIMARY KEY,
                  iv BLOB,
                  time INTEGER,
                  rounds INTEGER,
                  password_id BLOB
                  )",
                [],
            )
        });
    }

    #[allow(dead_code)]
    fn create_password(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS password (
                  id BLOB PRIMARY KEY,
                  name STRING,
                  checksum BLOB,
                  time INTEGER,
                  rounds INTEGER,
                  data    STRING,
                  iv BLOB
                  )",
                [],
            )
        });
    }

    #[allow(dead_code)]
    fn create_rewards(&self) -> rusqlite::Result<usize, Error> {
        return self.connection().and_then(|c| {
            c.execute(
                "CREATE TABLE IF NOT EXISTS rewards (
                  time BLOB PRIMARY KEY,
                  hash BLOB,
                  transaction BLOB
                  )",
                [],
            )
        });
    }

    pub fn select_latest_reward_hash(&self) -> Result<Vec<u8>, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare("SELECT hash FROM rewards ORDER BY time DESC LIMIT 1")?;
        let mut rows = statement.query_map(params![], |row| {
            let hash: Vec<u8> = row.get(0)?;
            Ok(hash)
        })?;
        Ok(rows.next().unwrap().unwrap())
    }

    pub fn select_reward_weights(&self) -> Result<Vec<RewardQueryResult>, Error> {
        let conn = self.connection()?;
        let mut statement =
            conn.prepare("SELECT reward_address, deterministic_trust FROM peers")?;
        let rows = statement.query_map(params![], |row| {
            let result = RewardQueryResult {
                reward_address: row.get(0)?,
                deterministic_trust: row.get(1)?,
            };
            Ok(result)
        })?;
        Ok(rows
            .filter(|x| x.is_ok())
            .map(|x| x.unwrap())
            .collect::<Vec<RewardQueryResult>>())
    }
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

    pub fn select_peer_id_from_key(&self, public_key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
        let conn = self.connection()?;
        let mut statement =
            conn.prepare("SELECT id FROM peer_key WHERE public_key = ?1 LIMIT 1")?;
        let rows = statement.query_map(params![public_key], |row| {
            let id: Vec<u8> = row.get(0)?;
            Ok(id)
        })?;
        let res = Ok(rows.filter(|x| x.is_ok()).map(|x| x.unwrap()).next());
        res
    }


    pub fn check_peer_connect(&self, peer_id: PeerId) -> Result<bool, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare(
            "SELECT COUNT(*) FROM peer_key JOIN peers ON peer_key.id = peers.id \
            WHERE peer_key.multi_hash = ?1 AND peers.trust > 0",
        )?;
        let mut rows = statement.query_map(params![peer_id.to_bytes()], |row| {
            let count: i64 = row.get(0)?;
            Ok(count > 0)
        })?;
        Ok(rows.next().unwrap().unwrap())
    }

    // Really neeed to figure out this parameter IN array binding stuff later.
    // for now just issuing multiple select statements for simplicity.
    pub fn select_peer_trust(
        &self,
        peer_ids: &Vec<Vec<u8>>,
    ) -> Result<HashMap<Vec<u8>, f64>, Error> {
        let conn = self.connection()?;
        let mut map: HashMap<Vec<u8>, f64> = HashMap::new();

        for peer_id in peer_ids {
            let mut statement = conn.prepare("SELECT id, trust FROM peers WHERE id = ?1")?;
            //.raw_bind_parameter();
            // Try this ^ for iterating over peer ids.
            let rows = statement.query_map(params![peer_id], |row| {
                Ok(PeerTrustQueryResult {
                    peer_id: row.get(0)?,
                    trust: row.get(1)?,
                })
            })?;
            // // TODO error handling for multiple rows

            for row in rows {
                let row_q = row?;
                map.insert(row_q.peer_id, row_q.trust);
            }
        }
        return Ok(map);
    }

    pub fn select_broadcast_peers(&self) -> rusqlite::Result<Vec<PeerQueryResult>, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare(
            "SELECT public_key, peers.trust FROM peer_key JOIN peers on peer_key.id = peers.id",
        )?;

        let rows = statement.query_map(params![], |row| {
            let result: f64 = row.get(1).unwrap_or(0 as f64);
            Ok(PeerQueryResult {
                public_key: row.get(0)?,
                trust: result,
            })
        })?;

        let res = rows.map(|r| r.unwrap()).collect::<Vec<PeerQueryResult>>();

        return Ok(res);
    }

    pub fn select_peer_key_address(
        &self,
        public_key: &Vec<u8>,
    ) -> rusqlite::Result<Option<String>, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare("SELECT address FROM peer_key WHERE public_key=?1")?;

        let rows = statement.query_map(params![public_key], |row| {
            let address: String = row.get(0)?;
            Ok(address)
        })?;

        let res = rows.map(|r| r.unwrap()).collect::<Vec<String>>();

        return Ok(res.get(0).map(|x| x.clone()));
    }

    pub fn map_err<A>(error: rusqlite::Result<A, rusqlite::Error>) -> result::Result<A, ErrorInfo> {
        error.map_err(|e| error_info(e.to_string()))
    }

    pub fn map_err_sqlx<A>(error: result::Result<A, sqlx::Error>) -> result::Result<A, ErrorInfo> {
        error.map_err(|e| error_message(schema::structs::Error::InternalDatabaseError, e.to_string()))
    }

    pub fn query_utxo_all_debug(&self) -> rusqlite::Result<Vec<UtxoEntry>, Error> {
        let conn = self.connection()?;
        let mut statement =
            conn.prepare("SELECT transaction_hash, output_index, address, output, time FROM utxo")?;

        let rows = statement.query_map(params![], |row| {
            Ok(UtxoEntry {
                transaction_hash: row.get(0)?,
                output_index: row.get(1)?,
                address: row.get(2)?,
                output: Some(Output::proto_deserialize(row.get(3)?).unwrap()),
                time: row.get(4)?,
            })
        })?;
        // TODO error handling for multiple rows
        let unwrapped = rows.map(|r| r.unwrap());
        return Ok(unwrapped.collect_vec());
    }

    pub fn query_all_balance(&self) -> rusqlite::Result<Vec<AddressBalance>, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare("SELECT address, output FROM utxo")?;

        let rows = statement.query_map(params![], |row| {
            let address_val: Vec<u8> = row.get(0)?;
            let output_raw: Vec<u8> = row.get(1)?;
            let output = Output::proto_deserialize(output_raw).unwrap();
            let rounded_balance = output.rounded_amount();
            Ok(AddressBalance {
                address: hex::encode(address_val),
                rounded_balance,
            })
        })?;
        // TODO error handling for multiple rows
        let mut totals: HashMap<String, f64> = HashMap::new();

        for row in rows {
            let row_q = row?;
            totals.insert(
                row_q.address.clone(),
                totals.get(&row_q.address.clone()).unwrap_or(&0.0) + row_q.rounded_balance,
            );
        }
        let mut res: Vec<AddressBalance> = vec![];
        for (k, v) in totals {
            res.push(AddressBalance {
                address: k,
                rounded_balance: v,
            })
        }
        return Ok(res);
    }

    pub fn query_utxo(
        &self,
        transaction_hash: &Vec<u8>,
        output_index: u32,
    ) -> rusqlite::Result<Option<UtxoEntry>, Error> {
        let conn = self.connection()?;

        let mut statement = conn.prepare(
            "SELECT output, time FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2",
        )?;

        let rows = statement.query_map(params![transaction_hash, output_index], |row| {
            let vec = row.get(0)?;
            let output = Output::proto_deserialize(vec).unwrap();
            let time: u64 = row.get(1)?;
            Ok(UtxoEntry::from_output(
                &output,
                transaction_hash,
                output_index as i64,
                time as i64,
            ))
        })?;
        for row in rows {
            let row_q = row?;
            return Ok(Some(row_q));
        }
        return Ok(None);
    }

    pub fn query_transaction(
        &self,
        transaction_hash: &Vec<u8>,
    ) -> rusqlite::Result<Option<Transaction>, Error> {
        let conn = self.connection()?;

        let mut statement =
            conn.prepare("SELECT raw_transaction FROM transactions WHERE hash = ?1")?;

        let rows = statement.query_map(params![transaction_hash], |row| {
            let vec = row.get(0)?;
            let output = Transaction::proto_deserialize(vec).unwrap();
            Ok(output)
        })?;
        for row in rows {
            let row_q = row?;
            return Ok(Some(row_q));
        }
        return Ok(None);
    }

    pub fn query_time_transaction(
        &self,
        start_time: u64,
        end_time: u64,
    ) -> rusqlite::Result<Vec<TransactionEntry>, Error> {
        let conn = self.connection()?;

        let mut statement = conn.prepare(
            "SELECT raw_transaction, time FROM transactions WHERE time >= ?1 AND time < ?2",
        )?;

        let rows = statement.query_map(params![start_time, end_time], |row| {
            let vec = row.get(0)?;
            let time: u64 = row.get(1)?;
            let transaction = Some(Transaction::proto_deserialize(vec).unwrap());
            Ok(TransactionEntry { transaction, time })
        })?;
        return Ok(rows.filter(|x| x.is_ok()).map(|x| x.unwrap()).collect_vec());
    }

    pub fn query_time_observation(
        &self,
        start_time: u64,
        end_time: u64,
    ) -> rusqlite::Result<Vec<ObservationEntry>, Error> {
        let conn = self.connection()?;

        let mut statement = conn.prepare(
            "SELECT raw_observation, time FROM observation WHERE time >= ?1 AND time < ?2",
        )?;

        let rows = statement.query_map(params![start_time, end_time], |row| {
            let vec = row.get(0)?;
            let time: u64 = row.get(1)?;
            let observation = Some(Observation::proto_deserialize(vec).unwrap());
            Ok(ObservationEntry { observation, time })
        })?;
        return Ok(rows.filter(|x| x.is_ok()).map(|x| x.unwrap()).collect_vec());
    }

    pub fn query_time_utxo(
        &self,
        start_time: u64,
        end_time: u64,
    ) -> rusqlite::Result<Vec<UtxoEntry>, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare(
            "SELECT transaction_hash, output_index, address, output, time \
            FROM utxo WHERE time >= ?1 AND time < ?2",
        )?;
        let rows = statement.query_map(params![start_time, end_time], |row| {
            Ok(UtxoEntry {
                transaction_hash: row.get(0)?,
                output_index: row.get(1)?,
                address: row.get(2)?,
                output: Some(Output::proto_deserialize(row.get(3)?).unwrap()),
                time: row.get(4)?,
            })
        })?;
        return Ok(rows.filter(|x| x.is_ok()).map(|x| x.unwrap()).collect_vec());
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

    pub fn get_max_time(&self, table: &str) -> rusqlite::Result<i64, Error> {
        let conn = self.connection()?;
        let query = "SELECT max(time) FROM ".to_owned() + table;
        let mut statement = conn.prepare(&*query)?;
        let mut rows = statement.query_map(params![], |r| {
            let data: i64 = r.get(0)?;
            Ok(data)
        })?;
        let row = rows.next().unwrap().unwrap_or(0 as i64);
        Ok(row)
    }

    pub fn query_download_times(&self) -> DownloadMaxTimes {
        DownloadMaxTimes {
            utxo: self.get_max_time("utxo").unwrap(),
            transaction: self.get_max_time("transactions").unwrap(),
            observation: self.get_max_time("observation").unwrap(),
            observation_edge: self.get_max_time("observation_edge").unwrap(),
        }
    }

    pub fn delete_utxo(
        &self,
        transaction_hash: &Vec<u8>,
        output_index: u32,
    ) -> rusqlite::Result<usize, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare(
            "DELETE FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2",
        )?;
        let rows = statement.execute(params![transaction_hash, output_index])?;
        return Ok(rows);
    }

    pub fn query_balance(&self, address: &Vec<u8>) -> rusqlite::Result<u64, Error> {
        let conn = self.connection()?;
        let mut statement = conn.prepare("SELECT output FROM utxo WHERE address = ?1")?;

        let rows = statement.query_map(params![address], |row| {
            let data: Vec<u8> = row.get(0)?;
            Ok(Output::proto_deserialize(data).unwrap().amount())
        })?;

        let total: u64 = rows.map(|r| r.unwrap()).sum();

        return Ok(total);
    }

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
            connection_path: path.clone(),
            pool: pl.clone(),
            address_block_store: AddressBlockStore{ ctx: ctx.clone() },
            peer_store: PeerStore{ ctx: ctx.clone() },
            config_store: ConfigStore{ ctx: ctx.clone() },
            // server_store: ServerStore{ ctx: ctx.clone() },
            transaction_store: TransactionStore{ ctx: ctx.clone() },
            multiparty_store: MultipartyStore { ctx: ctx.clone() },
            observation: ObservationStore { ctx: ctx.clone() }
        }
    }

    pub async fn in_memory() -> DataStore {
        DataStore::from_path("file:memdb1?mode=memory&cache=shared".parse().unwrap()).await
    }

    pub async fn in_memory_idx(id: u16) -> DataStore {
        let path = "file:memdb1_id".to_owned() + &*id.to_string() + "?mode=memory&cache=shared";
        DataStore::from_path(path).await
    }

    pub async fn from_config(node_config: &NodeConfig) -> DataStore {
        DataStore::from_path(format!("{}{}", "file:", node_config.data_store_path)).await
    }

    pub async fn from_file_path(path: String) -> DataStore {
        DataStore::from_path(format!("{}{}", "file:", path)).await
    }


    pub fn create_all_err(&self) -> rusqlite::Result<Connection, Error> {
        let c = self.connection()?;
        self.create_debug_table()?;
        // self.create_peers()?;
        // self.create_peer_keys()?;
        // self.create_transactions()?;
        // self.create_observation()?;
        // self.create_observation_edge()?;
        //
        // sqlx::migrate!("./migrations")
        //     .run(&*self.pool)
        //     .await
        //     .expect("Migrations failure");

        return Ok(c);
    }

    pub async fn run_migrations(&self) -> result::Result<(), ErrorInfo> {
        sqlx::migrate!("./migrations")
            .run(&*self.pool)
            .await
            .map_err(|e| error_message(schema::structs::Error::InternalDatabaseError, e.to_string()))
    }

    pub fn create_all_err_info(&self) -> Result<Connection, ErrorInfo> {
        DataStore::map_err(self.create_all_err())
    }
}

// "file:memdb1?mode=memory&cache=shared"
// #[tokio::test]
// async fn better_example_sqlite() {
//     // let tc = TestConstants::new();
//
//     let ds = DataStore::in_memory().await;
//     // ds.create_mnemonic().unwrap();
//     ds.create_transactions().unwrap();
//     let _c = ds
//         .create_all_err()
//         //.await
//         .unwrap();
//     ds.create_utxo().expect("ignore");
//     let ex = utxo::get_example_utxo_entry();
//     ds.insert_utxo(&ex).unwrap();
//     let result = ds.query_utxo(&ex.transaction_hash, ex.output_index as u32);
//     let entry = result.unwrap().unwrap();
//     assert_eq!(ex, entry);
//
//     println!("{:?}", ds.query_download_times());
// }
//
// #[tokio::test]
// async fn example_insert_transaction() {
//     // let tc = TestConstants::new();
//     let ds = DataStore::in_memory().await;
//     let _c = ds
//         .create_all_err()
//         //.await
//         .unwrap();
//     println!("wtf: {:?}", ds.select_all_tables());
//     let g = create_genesis_transaction();
//     ds.insert_transaction(&g.clone(), EARLIEST_TIME).unwrap();
//     let q = ds.query_transaction(&g.hash_vec());
//     assert_eq!(q.unwrap().unwrap(), g);
//     // let result = ds.query_utxo(&g.transaction_hash, ex.output_index);
//     // let entry = result.unwrap().unwrap();
// }
//
// #[tokio::test]
// async fn peers_example() {
//     let tc = TestConstants::new();
//     let ds = DataStore::in_memory().await;
//     let _c = ds
//         .create_all_err_info()
//         // .await
//         .expect("work");
//     for i in 0..10 {
//         ds.insert_peer(tc.peer_ids.get(i).unwrap(), *tc.peer_trusts.get(i).unwrap())
//             .expect("fix");
//         // println!("res {:?}", res)
//     }
//
//     let res = ds.select_peer_trust(&tc.peer_ids[0..3].to_vec()).unwrap();
//     for i in 0..3 {
//         let id = tc.peer_ids.get(i).unwrap();
//         let trust = tc.peer_trusts.get(i).unwrap();
//         assert_eq!(res.get(id).unwrap(), trust)
//     }
// }
//
// // "file:memdb1?mode=memory&cache=shared"
// #[test]
// fn example_sqlite() -> Result<()> {
//     let conn = Connection::open("file:memdb1_test?mode=memory&cache=shared")?;
//     // let conn = Connection::open_in_memory()?;
//     /*
//        id: Vec<u8>,
//        address: Vec<u8>,
//        amount: u64,
//        weights: Vec<f64>,
//        threshold: Option<f64>
//     */
//     conn.execute(
//         "CREATE TABLE IF NOT EXISTS utxo (
//                   id    BLOB PRIMARY KEY,
//                   address            BLOB,
//                   data            BLOB,
//                   contract            BLOB,
//                   weights            BLOB,
//                   threshold            DOUBLE
//                   )",
//         [],
//     )?;
//
//     let me = UtxoEntry {
//         id: UtxoEntry::id_from_fixed(&FixedIdConvert::from_values(&util::sha256_str("asdf"), 500)),
//         address: address(&TestConstants::new().public).to_vec(),
//         data: CurrencyData { amount: 100 }.proto_serialize(),
//         weights: vec![],
//         threshold: None,
//         contract: Transaction::currency_contract_hash(),
//     };
//     conn.execute(
//         "INSERT INTO utxo (id, address, data, contract, weights, threshold) VALUES (?1, ?2, ?3, ?4, ?5)",
//         params![me.id, me.address, me.data, me.contract, me.weights, me.threshold],
//     )?;
//
//     let mut stmt =
//         conn.prepare("SELECT id, address, data, contract, weights, threshold FROM utxo")?;
//     let person_iter = stmt.query_map([], |row| {
//         Ok(UtxoEntry {
//             id: row.get(0)?,
//             address: row.get(1)?,
//             data: row.get(2)?,
//             contract: row.get(3)?,
//             weights: row.get(4)?,
//             threshold: row.get(5)?,
//         })
//     })?;
//
//     for person in person_iter {
//         let result = person.unwrap();
//         println!("Found person {:?}", result.id);
//         println!("Found person {:?}", result.address);
//         println!("Found person {:?}", result.data);
//     }
//     Ok(())
// }

// https://stackoverflow.com/questions/64420765/how-can-i-deserialize-a-prost-enum-with-serde

// #[test]
// fn enum_ser_test() {
//     // let ht = HashType::Transaction;
// }

#[allow(dead_code)]
#[derive(sqlx::FromRow)]
pub struct MnemonicEntry {
    pub(crate) words: String,
    pub(crate) time: i64,
    pub(crate) peer_id: Vec<u8>,
}

//
// async fn add_todo(pool: &SqlitePool, description: String) -> anyhow::Result<i64> {
//
//     Ok(id)
// }

#[tokio::test]
async fn test_mnemonic() {
    let ds = DataStore::in_memory().await;
    ds.create_mnemonic().await.expect("created");
    ds.insert_mnemonic(MnemonicEntry {
        words: "asdf".to_string(),
        time: 0,
        peer_id: vec![],
    })
    .await
    .expect("insert");
    let res = ds.query_all_mnemonic().await;
    let vec1 = res.expect("query");
    let m = vec1.get(0).expect("0").clone();
    assert_eq!(m.words, "asdf".to_string());
}

// Fix later
#[ignore]
#[tokio::test]
async fn test_sqlx() {
    // use sqlx::Connection;
    // DATABASE_URL=sqlite:///Users/test_sqlite.sqlite
    let path = DataStore::in_memory().await.connection_path;
    //std::env::set_var("DATABASE_URL", path.clone());
    let tbl = "CREATE TABLE IF NOT EXISTS mnemonic (
                  words    STRING PRIMARY KEY,
                  time INTEGER,
                  peer_id BLOB
                  )";

    let pool = SqlitePool::connect(&*path.clone())
        .await
        .expect("Connection failure");
    let mut conn = pool.acquire().await.expect("acquire failure");
    // conn.
    let _res = sqlx::query(tbl)
        .fetch_all(&mut conn)
        .await
        .expect("create failure");

    let _res2 = sqlx::query("INSERT INTO mnemonic (words, time, peer_id) VALUES (?, ?, ?)")
        .bind("yo")
        .bind(0 as i64)
        .bind(util::sha256_str("sadf").to_vec())
        .fetch_all(&mut conn)
        .await
        .expect("create failure");

    let res3 = sqlx::query("select words, time, peer_id from mnemonic")
        .fetch_all(&mut conn)
        .await
        .expect("create failure");
    let res4 = res3.get(0).expect("something");

    let wordss: &str = res4.try_get("words").expect("yes");
    println!("{:?}", wordss);
    assert_eq!(wordss, "yo");
    // Insert the task, then obtain the ID of this row
    //     let vecu8 = util::sha256_str("sadf").to_vec();
    //     let id = sqlx::query!(
    //         r#"
    // INSERT INTO mnemonic ( words, time, peer_id )
    // VALUES ( ?1, ?2, ?3)
    //         "#,
    //         "yo",
    //         0 as u64,
    //     )
    //     .execute(&mut conn)
    //     .await?
    //     .last_insert_rowid();
    //     println!("{:?}", id);

    // let conn = sqlx::SqliteConnection::connect(&*path)
    //     .await
    //     .expect("Gimme");

    //
    // let mut stream = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ? OR name = ?")
    //     .bind(user_email)
    //     .bind(user_name)
    //     .fetch(&mut conn);

    /*

    */
}

#[derive(Debug)]
struct Todo {
    id: i64,
    description: String,
    done: bool,
}

#[tokio::test]
async fn test_sqlx_migrations() {
    // TODO: Delete file at beginning.
    dotenv::dotenv().ok().expect("worked");
    util::init_logger().ok(); //expect("log");
    println!("{:?}", std::env::var("DATABASE_URL"));
    let mut node_config = NodeConfig::default();
    let mut args = empty_args();
    args.network = Some("debug".to_string());
    node_config = crate::util::cli::arg_parse_config::load_node_config_initial(args, node_config);

    println!(
        "{:?}",
        std::fs::remove_file(Path::new(&node_config.data_store_path)).is_ok()
    );

    // node_config = NodeConfig::new(&(0 as u16));
    println!("{:?}", node_config.data_store_path);

    let ds = // "sqlite:///Users//.rg/debug/data_store.sqlite".to_string()
        // DataStore::from_path(&node_config).await;
        DataStore::from_config(&node_config).await;
    // let pool = SqlitePool::connect("sqlite:///Users//.rg/debug/data_store.sqlite")
    //     .await
    //     .expect("Connection failure");
    let mut conn = ds.pool.acquire().await.expect("connection");

    // this works
    sqlx::migrate!("./migrations")
        .run(&*ds.pool)
        .await
        .expect("Wtf");

    let _res2 = sqlx::query("INSERT INTO todos (id, description, done) VALUES (?, ?, ?)")
        .bind(1)
        .bind("whoa some description here")
        .bind(true)
        .fetch_all(&mut conn)
        .await
        .expect("create failure");

    let res3 = sqlx::query("select id, description, done from todos")
        .fetch_all(&mut conn)
        .await
        .expect("create failure");
    let res4 = res3.get(0).expect("something");

    let wordss: &str = res4.try_get("description").expect("yes");
    println!("{:?}", wordss);
    assert_eq!(wordss, "whoa some description here");

    let ress: Vec<Todo> = sqlx::query_as!(Todo, "select id, description, done from todos")
        .fetch_all(&*ds.pool) // -> Vec<Country>
        .await
        .expect("worx");

    println!("{:?}", ress);

    /*
             r#"
    UPDATE todos
    SET done = TRUE
    WHERE id = ?1
            "#,
            id
         */
}
