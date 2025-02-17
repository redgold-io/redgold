use crate::schema::SafeOption;
use crate::DataStoreContext;
use itertools::Itertools;
use metrics::{counter, gauge};
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::TestConstants;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::PeerIdInfo;
use redgold_schema::structs::{ErrorInfo, Hash, NodeMetadata, PartitionInfo, PeerId, PeerMetadata, PeerNodeInfo, PublicKey, Transaction};
use redgold_schema::util::times;
use redgold_schema::util::xor_distance::xorfc_hash;
use redgold_schema::{util, RgResult};
use std::time::Duration;
use tracing::error;

#[derive(Clone)]
pub struct PeerStore {
    pub ctx: DataStoreContext
}

#[derive(Clone)]
pub struct PeerTrustQueryResult {
    pub peer_id: PeerId,
    pub trust: f64,
}

#[derive(Clone)]
pub struct PeerIdNode {
    pub peer_id: PeerId,
    pub public_key: PublicKey,
}


impl PeerStore {

    // This should actually just be a join based select.
    pub async fn all_peers_info(&self) -> RgResult<Vec<PeerNodeInfo>> {
        let nodes = self.nodes_tx().await?;
        // let peers = self.all_peers_tx().await?;
        let mut res = vec![];
        for n in nodes {
            if let Some(pid) = n.node_metadata()?.peer_id {
                let tx = self.query_peer_id_tx(&pid).await?;
                if let Some(t) = tx {
                    res.push(PeerNodeInfo {
                        latest_peer_transaction: Some(t),
                        latest_node_transaction: Some(n),
                        dynamic_node_metadata: None,
                    })
                }
            }
        }
        Ok(res)
    }

    pub async fn remove_peer_id(&self, p: &PeerId) -> RgResult<()> {
        let mut pool = self.ctx.pool().await?;
        let vec = p.peer_id.safe_get()?.vec();
        let rows = sqlx::query!("DELETE FROM peers WHERE id = ?1", vec)
            .execute(&mut *pool)
            .await;
        let _rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(())
    }


    // TODO: Implement XOR distance + fee distance metrics + trust distance metrics
    pub async fn select_gossip_peers(&self, _tx: &Transaction) -> Result<Vec<PublicKey>, ErrorInfo> {
        self.active_nodes(None).await
    }
    pub async fn peers_near(&self, hash: &Hash, partition: fn(&PartitionInfo) -> Option<i64>) -> Result<Vec<PublicKey>, ErrorInfo> {
        let map = self.active_node_info(None).await?.iter()
            .filter(|&p|
                p.node_metadata()
                    .map(|n| n.hash_in_range(hash, partition)).unwrap_or(false)
            ).flat_map(|p| p.node_metadata().ok().and_then(|n| n.public_key))
            .collect_vec();
        Ok(map)
    }

    pub async fn query_public_key_metadata(
        &self,
        public: &PublicKey,
    ) -> Result<Option<NodeMetadata>, ErrorInfo> {
        Ok(self.query_public_key_node(public).await?
            .and_then(|v| v.node_metadata().ok())
        )
    }

    pub async fn query_peer_id_info(
        &self,
        peer_id: &PeerId,
    ) -> Result<Option<PeerIdInfo>, ErrorInfo> {
        let tx = self.query_peer_id_tx(peer_id).await?;
        let mut res = vec![];
        if let Some(tx) = &tx{
            for nmd in tx.peer_data()?.node_metadata {
                if let Some(pk) = &nmd.public_key {
                    if let Some(v) = self.query_public_key_node(pk).await? {
                        res.push(v);
                    }
                }
            };
        }
        if let Some(tx) = tx {
            let res = res.iter().map(|r| PeerNodeInfo{
                latest_peer_transaction: Some(tx.clone()),
                latest_node_transaction: Some(r.clone()),
                dynamic_node_metadata: None,
            }).collect_vec();
            return Ok(Some(PeerIdInfo {
                latest_peer_transaction: Some(tx),
                peer_node_info: res
            }))
        }
        Ok(None)
    }

    pub async fn query_peer_id_tx(
        &self,
        peer_id: &PeerId,
    ) -> Result<Option<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let x = peer_id;
        let vec = x.vec();
        let hx = hex::encode(vec.clone());

        let rows = sqlx::query!(
            r#"SELECT tx FROM peers WHERE id = ?1"#,
            vec
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;

        if let Some(rows) = rows_m {
            let pni: Vec<u8> = rows.tx.clone();
            return Ok(Some(Transaction::proto_deserialize(pni)?));
        }
        Ok(None)
    }
    pub async fn update_last_seen(&self, node: &PublicKey) -> Result<(), ErrorInfo> {

        let res = self.query_public_key_node(node).await?;
        if res.is_none() {
            return Err(ErrorInfo::error_info("Missing node in database"));
        }

        let mut pool = self.ctx.pool().await?;

        let bytes = node.vec();
        let time = times::current_time_millis();
        let rows = sqlx::query!(
            r#"UPDATE nodes SET last_seen = ?1 WHERE public_key = ?2"#,
            time,
            bytes
        ) // Execute instead of fetch
            .execute(&mut *pool)
            .await;
        let rows_affected = DataStoreContext::map_err_sqlx(rows)?;
        let rows_affected = rows_affected.rows_affected();
        counter!("redgold_peer_store_update_last_seen").increment(rows_affected);
        Ok(())
    }

    // TODO: Add node transaction as well
    // pub async fn add_peer(&self, tx: &Transaction, trust: f64) -> Result<(), ErrorInfo> {
    //     // return Err(ErrorInfo::error_info("debug error return"));
    //     let pd = tx.peer_data()?;
    //     let tx_blob = tx.proto_serialize();
    //     let pd_blob = pd.proto_serialize();
    //     let tx_hash = tx.hash().vec();
    //     let mut pool = self.ctx.pool().await?;
    //     let pid = pd.peer_id.safe_get()?.clone().peer_id.safe_get()?.clone().value;
    //
    //     let rows = sqlx::query!(
    //         r#"INSERT OR REPLACE INTO peers (id, peer_data, tx, trust, tx_hash) VALUES (?1, ?2, ?3, ?4, ?5)"#,
    //         pid,
    //         pd_blob,
    //         tx_blob,
    //         trust,
    //         tx_hash
    //     )
    //         .fetch_all(&mut *pool)
    //         .await;
    //     let _ = DataStoreContext::map_err_sqlx(rows)?;
    //     let time = util::current_time_millis();
    //
    //     for nmd in pd.node_metadata {
    //         let ser = nmd.proto_serialize();
    //         let rows = sqlx::query!(
    //         r#"INSERT OR REPLACE INTO peer_key (public_key, id, multi_hash, address, status, last_seen, node_metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
    //         nmd.public_key.safe_get()?.bytes.safe_get()?.value,
    //         pid,
    //         nmd.multi_hash,
    //         nmd.external_address,
    //         "",
    //             time,
    //             ser
    //         )
    //             .fetch_all(&mut *pool)
    //             .await;
    //         let _ = DataStoreContext::map_err_sqlx(rows)?;
    //
    //     }
    //     Ok(())
    // }


    pub async fn insert_peer(&self, tx: &Transaction) -> Result<i64, ErrorInfo> {
        let pd = tx.peer_data()?;
        let tx_blob = tx.proto_serialize();
        let mut pool = self.ctx.pool().await?;
        let pid = pd.peer_id.safe_get()?.vec();

        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peers (id, tx) VALUES (?1, ?2)"#,
            pid,
            tx_blob,
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn add_peer_new(&self, peer_info: &PeerNodeInfo, self_key: &PublicKey) -> Result<(), ErrorInfo> {
        // return Err(ErrorInfo::error_info("debug error return"));
        // tracing::info!("add_peer_new");
        if peer_info.public_keys().contains(&self_key) {
            error!("Self key found in peer info {}", peer_info.json_or());
            self.remove_node(&self_key).await?;
            return Ok(())
        }

        self.insert_peer(
            peer_info
                .latest_peer_transaction
                .safe_get_msg("Add peer failed due to missing latest peer transaction")?
        ).await?;
        self.insert_node(peer_info.latest_node_transaction.safe_get_msg("Missing peer info latest node tx")?).await?;
        Ok(())
    }

    pub async fn all_peers_tx(
        &self
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT tx FROM peers"#
        )
            .fetch_all(&mut *pool)
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
    ) -> Result<Vec<PeerMetadata>, ErrorInfo> {
        self.all_peers_tx().await?.iter().map(|tx| tx.peer_data()).collect()
    }

    pub async fn active_nodes(
        &self,
        delay: Option<Duration>
    ) -> Result<Vec<PublicKey>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let delay = delay.unwrap_or(Duration::from_secs(60*60*24));
        let delay = delay.as_millis() as i64;
        let cutoff = times::current_time_millis() - delay;
        let cutoff = 0i64;

        let rows = sqlx::query!(
            r#"SELECT public_key FROM nodes WHERE last_seen > ?1"#,
            cutoff
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = PublicKey::proto_deserialize(
                row.public_key.clone()
            )?;
            deser.validate()?;
            res.push(deser);
        }
        Ok(res)
    }

    pub async fn active_nodes_ids(
        &self,
        delay: Option<Duration>
    ) -> Result<Vec<PeerIdNode>, ErrorInfo> {
        let delay = delay.unwrap_or(Duration::from_secs(60*60*24));
        let delay = delay.as_millis() as i64;
        let cutoff = times::current_time_millis() - delay;
        let cutoff = 0i64;

        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT public_key, peer_id FROM nodes WHERE last_seen > ?1"#,
            cutoff
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?.into_iter().map(
            |r| PeerId::proto_deserialize_ref(r.peer_id.as_ref())
                .and_then(|peer_id| PublicKey::proto_deserialize_ref(r.public_key.as_ref())
                    .map(|public_key| PeerIdNode {
                        peer_id,
                        public_key
                    }))).collect()
    }


    pub async fn active_node_metadata(
        &self,
        delay: Option<Duration>
    ) -> Result<Vec<NodeMetadata>, ErrorInfo> {
        let res = self.active_node_info(delay).await?
            .iter()
            .filter_map(|pni|
                            pni.node_metadata().ok()
            ).collect_vec();
        Ok(res)
    }


}

// Peer Node Key Store Functions
impl PeerStore {

    pub async fn nodes_tx(&self) -> Result<Vec<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT tx FROM nodes"#
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.tx)?;
            res.push(deser);
        }
        Ok(res)
    }

    pub async fn active_node_info(
        &self,
        delay: Option<Duration>
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let delay = delay.unwrap_or(Duration::from_secs(60*60*24));
        let delay = delay.as_millis() as i64;
        let cutoff = times::current_time_millis() - delay;
        let cutoff = 0i64;

        // TODO: Fix root cause of last_seen not being update
        let rows = sqlx::query!(
            r#"SELECT tx FROM nodes WHERE last_seen > ?1"#,
            cutoff
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.tx)?;
            res.push(deser);
        }
        Ok(res)
    }

    pub async fn peer_id_for_node_pk(&self, public_key: &PublicKey) -> RgResult<Option<PeerId>> {
        let mut pool = self.ctx.pool().await?;
        let vec = public_key.validate()?.vec();
        let rows = sqlx::query!(
            r#"SELECT peer_id FROM nodes WHERE public_key = ?1"#,
            vec
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        if let Some(r) = rows_m {
            Ok(Some(PeerId::from_bytes(r.peer_id)?))
        } else {
            Ok(None)
        }
    }

    pub async fn peer_id_count_node_pk(&self, public_key: &PublicKey) -> RgResult<i32> {
        let mut pool = self.ctx.pool().await?;
        let vec = public_key.validate()?.vec();
        let rows = sqlx::query!(
            r#"SELECT count(peer_id) as count FROM nodes WHERE public_key = ?1"#,
            vec
        )
            .fetch_one(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.count)
    }


    pub async fn remove_node(&self, p0: &PublicKey) -> RgResult<()> {
        counter!("redgold_peer_store_remove_node").increment(1);
        let pid = self.peer_id_for_node_pk(p0).await?;
        if let Some(p) = pid {
            let mut pool = self.ctx.pool().await?;
            let vec = p0.validate()?.vec();
            let rows = sqlx::query!("DELETE FROM nodes WHERE public_key = ?1", vec)
                .execute(&mut *pool)
                .await;
            let num_peers = DataStoreContext::map_err_sqlx(rows)?.rows_affected();
            counter!("redgold_peer_store_nodes_deleted").increment(num_peers);
            let c = self.peer_id_count_node_pk(p0).await?;
            if c > 0 {
                self.remove_peer_id(&p).await?;
            }

        }
        Ok(())
    }

    pub async fn clear_all_peers(&self) -> RgResult<()> {
        let mut pool = self.ctx.pool().await?;
        let _rows = DataStoreContext::map_err_sqlx(sqlx::query!("DELETE FROM nodes")
            .execute(&mut *pool)
            .await)?.rows_affected();

        let _rows = DataStoreContext::map_err_sqlx(sqlx::query!("DELETE FROM peers")
            .execute(&mut *pool)
            .await)?.rows_affected();

        Ok(())
    }

    pub async fn query_public_key_node(
        &self,
        public: &PublicKey,
    ) -> Result<Option<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let vec = public.validate()?.vec();
        let rows = sqlx::query!(
            r#"SELECT tx FROM nodes WHERE public_key = ?1"#,
            vec
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        if let Some(rows) = rows_m {
            return Ok(Some(Transaction::proto_deserialize(rows.tx)?));
        }
        Ok(None)
    }

    pub async fn query_nodes_peer_node_info(
        &self,
        public: &PublicKey,
    ) -> Result<Option<PeerNodeInfo>, ErrorInfo> {
        let tx = self.query_public_key_node(public).await?;
        let pid = tx.as_ref()
            .and_then(|tx| tx.node_metadata().ok().and_then(|n| n.peer_id));
        if let (Some(tx), Some(pid)) = (tx, pid) {
            if let Some(p) = self.query_peer_id_tx(&pid).await? {
                return Ok(Some(PeerNodeInfo{
                    latest_peer_transaction: Some(p),
                    latest_node_transaction: Some(tx),
                    dynamic_node_metadata: None,
                }));
            }
        }
        Ok(None)
    }

    pub async fn insert_node(&self, tx: &Transaction) -> Result<i64, ErrorInfo> {
        let time = times::current_time_millis();
        let mut pool = self.ctx.pool().await?;
        let nmd = tx.node_metadata()?;
        let public_key = nmd.public_key.safe_get()?;
        public_key.validate()?;
        let pk_bytes = public_key.vec();
        let pid = nmd.peer_id.safe_get()?.vec();
        let _ser_nmd = nmd.proto_serialize();
        let tx_ser = tx.proto_serialize();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO nodes (public_key, peer_id, status, last_seen, tx) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            pk_bytes,
            pid,
            "",
            time,
            tx_ser
            )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }



}

#[test]
fn distance_check(){
    let tc = TestConstants::new();
    let a = tc.rhash_1.vec();
    let b = tc.rhash_2.vec();

    let _c: Vec<u8> = a.iter().zip(b).map(|(x, y)| x ^ y).collect();


}