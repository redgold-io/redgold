use std::time::Duration;
use redgold_schema::structs::{Address, Error, ErrorInfo, Hash, PeerData, PeerNodeInfo, PublicKey, Transaction};
use redgold_schema::{ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants, util, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct PeerStore {
    pub ctx: DataStoreContext
}


impl PeerStore {

    /*
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

     */

    // pub async fn public_key_trust(&self) -> Result<>

    pub async fn update_last_seen(&self, node: PublicKey) -> Result<(), ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let bytes = node.bytes.safe_bytes()?;
        let time = util::current_time_millis();
        let rows = sqlx::query!(
            r#"UPDATE peer_key SET last_seen = ?1 WHERE public_key = ?2"#,
            time,
            bytes
        ) // Execute instead of fetch
            .fetch_all(&mut pool)
            .await;
        let _ = DataStoreContext::map_err_sqlx(rows)?;
        Ok(())
    }

    // TODO: Add node transaction as well
    pub async fn add_peer(&self, tx: &Transaction, trust: f64) -> Result<(), ErrorInfo> {
        // return Err(ErrorInfo::error_info("debug error return"));
        let pd = tx.peer_data()?;
        let tx_blob = tx.proto_serialize();
        let pd_blob = pd.proto_serialize();
        let tx_hash = tx.hash().vec();
        let mut pool = self.ctx.pool().await?;
        let pid = pd.peer_id.safe_get()?.clone().peer_id.safe_get()?.clone().value;

        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peers (id, peer_data, tx, trust, tx_hash) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            pid,
            pd_blob,
            tx_blob,
            trust,
            tx_hash
        )
            .fetch_all(&mut pool)
            .await;
        let _ = DataStoreContext::map_err_sqlx(rows)?;
        let time = util::current_time_millis();

        for nmd in pd.node_metadata {
            let ser = nmd.proto_serialize();
            let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peer_key (public_key, id, multi_hash, address, status, last_seen, node_metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            nmd.public_key.safe_get()?.bytes.safe_get()?.value,
            pid,
            nmd.multi_hash,
            nmd.external_address,
            "",
                time,
                ser
            )
                .fetch_all(&mut pool)
                .await;
            let _ = DataStoreContext::map_err_sqlx(rows)?;

        }
        Ok(())
    }


    pub async fn insert_peer(&self, tx: &Transaction, trust: f64) -> Result<i64, ErrorInfo> {
        let pd = tx.peer_data()?;
        let tx_blob = tx.proto_serialize();
        let pd_blob = pd.proto_serialize();
        let tx_hash = tx.hash().vec();
        let mut pool = self.ctx.pool().await?;
        let pid = pd.peer_id.safe_get()?.clone().peer_id.safe_get()?.clone().value;

        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peers (id, peer_data, tx, trust, tx_hash) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            pid,
            pd_blob,
            tx_blob,
            trust,
            tx_hash
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn insert_node_key(&self, pi: &PeerNodeInfo) -> Result<i64, ErrorInfo> {
        let tx = pi.latest_node_transaction.safe_get()?;
        let time = util::current_time_millis();
        let mut pool = self.ctx.pool().await?;
        let nmd = tx.node_metadata()?;
        let pid = nmd.peer_id.safe_get()?.peer_id.safe_bytes()?;
        let ser = nmd.proto_serialize();
        let pni = pi.proto_serialize();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peer_key (public_key, id, multi_hash, address, status, last_seen, node_metadata, peer_node_info) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            nmd.public_key.safe_get()?.bytes.safe_get()?.value,
            pid,
            nmd.multi_hash,
            nmd.external_address,
            "",
                time,
                ser,
                pni
            )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn add_peer_new(&self, peer_info: &PeerNodeInfo, trust: f64) -> Result<(), ErrorInfo> {
        // return Err(ErrorInfo::error_info("debug error return"));
        self.insert_peer(peer_info.latest_peer_transaction.safe_get()?, trust).await?;
        self.insert_node_key(peer_info).await?;
        Ok(())
    }

    pub async fn peer_node_info(&self) -> Result<Vec<PeerNodeInfo>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT peer_node_info FROM peer_key"#
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            if let Some(r) = row.peer_node_info {
                let deser = PeerNodeInfo::proto_deserialize(r)?;
                res.push(deser);
            }
        }
        Ok(res)
    }

    pub async fn all_peers_tx(
        &self
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT tx FROM peers"#
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.tx)?;
            res.push(deser);
        }
        Ok(res)
    }

    pub async fn all_peers_nodes(
        &self
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT tx FROM peers"#
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.tx)?;
            res.push(deser);
        }
        Ok(res)
    }

    pub async fn all_peers(
        &self
    ) -> Result<Vec<PeerData>, ErrorInfo> {
        self.all_peers_tx().await?.iter().map(|tx| tx.peer_data()).collect()
    }

    pub async fn active_nodes(
        &self,
        delay: Option<Duration>
    ) -> Result<Vec<PublicKey>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let delay = delay.unwrap_or(Duration::from_secs(60*60*24));
        let delay = delay.as_millis() as i64;
        let cutoff = util::current_time_millis() - delay;

        let rows = sqlx::query!(
            r#"SELECT id FROM peer_key WHERE last_seen > ?1"#,
            cutoff
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = PublicKey::from_bytes(
                row.id.safe_get_msg("Missing public key in database")?.clone()
            );
            res.push(deser);
        }
        Ok(res)
    }
    //
    // pub async fn peers_by_utxo_distance(
    //     &self,
    //     hash: Hash,
    // ) -> Result<usize, ErrorInfo> {
    //     let mut pool = self.ctx.pool().await?;
    //     // let rows = sqlx::query!(
    //     //     r#"SELECT UNIQUE(peers.peer_data) FROM peer_key INNER JOIN on peer_key.id = peers.id WHERE peer_key.id"#,
    //     //     height
    //     // )
    //     //     .fetch_all(&mut pool)
    //     //     .await;
    //     // let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     // let mut res = vec![];
    //     // for row in rows_m {
    //     //     res.push(AddressHistoricalBalance{
    //     //         address: Address::from_bytes(row.address.safe_get_msg("Row missing address")?.clone())?,
    //     //         balance: row.balance.safe_get_msg("Row missing balance")?.clone(),
    //     //         height: height.clone()
    //     //     });
    //     // }
    //     // Ok(res)
    //     Ok(0)
    // }

}

#[test]
fn distance_check(){
    let tc = TestConstants::new();
    let a = tc.rhash_1.safe_bytes().unwrap();
    let b = tc.rhash_2.safe_bytes().unwrap();

    let c: Vec<u8> = a.iter().zip(b).map(|(x, y)| x ^ y).collect();


}