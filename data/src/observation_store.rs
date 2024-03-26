use itertools::Itertools;
use redgold_keys::TestConstants;
use redgold_schema::structs::{ErrorInfo, Hash, Observation, ObservationEdge, ObservationEntry, ObservationProof, PublicKey, Transaction, TransactionEntry};
use redgold_schema::{ProtoHashable, ProtoSerde, RgResult, SafeBytesAccess, structs, util, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct ObservationStore {
    pub ctx: DataStoreContext,
}

impl ObservationStore {
    pub async fn count_total_observations(
        &self
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM observation"#
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            res.push(row.count as i64);
        }
        let option = res.get(0).safe_get()?.clone().clone();
        Ok(option)
    }

    pub async fn select_latest_observation(&self, peer_key: PublicKey) -> Result<Option<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let bytes = peer_key.bytes()?;
        let rows = sqlx::query!(
            r#"SELECT raw FROM observation WHERE public_key = ?1 ORDER BY height DESC LIMIT 1"#,
            bytes
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw;
            let deser = Transaction::proto_deserialize(option1)?;
            res.push(deser);
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

    pub async fn get_pk_observations(&self, node_pk: &PublicKey, limit: i64) -> Result<Vec<Transaction>, ErrorInfo> {
        let bytes = node_pk.bytes()?;
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM observation WHERE public_key = ?1 ORDER BY height DESC LIMIT ?2"#,
            bytes,
            limit
        ).fetch_all(&mut *self.ctx.pool().await?).await)?
            .iter().map(|o| Transaction::proto_deserialize_ref(&o.raw)).collect()
    }

    pub async fn insert_observation(
        &self,
        observation_tx: &Transaction,
        time: i64,
        tx_hash: &Hash,
        height: i64,
        public_key: &PublicKey
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let hash = tx_hash.safe_bytes()?;
        let ser = observation_tx.proto_serialize();
        let public_key = public_key.bytes.safe_bytes()?.clone();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO observation
            (hash, raw, public_key, time, height) VALUES
            (?1, ?2, ?3, ?4, ?5)"#,
            hash,
            ser,
            public_key,
            time,
            height
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn query_time_observation(&self, start_time: i64, end_time: i64) -> Result<Vec<ObservationEntry>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw, time FROM observation WHERE time >= ?1 AND time <= ?2"#,
            start_time,
            end_time
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw;
            let deser = Transaction::proto_deserialize(option1)?;
            let time = row.time.clone();
            let mut entry = ObservationEntry::default();
            entry.observation = Some(deser);
            entry.time = time as i64;
            res.push(entry);
        }
        Ok(res)
    }

    pub async fn accepted_time_observation_hashes(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Hash>> {
        let rows = DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT hash FROM observation WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        ).fetch_all(&mut *self.ctx.pool().await?)
            .await)?.into_iter().flat_map(|row| row.hash.map(|h| Hash::new(h))).collect_vec();
        Ok(rows)
    }

    pub async fn query_observation(&self, hash: &Hash) -> RgResult<Option<Transaction>> {
        let hash = hash.safe_bytes()?;
        let rows =  DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM observation WHERE hash = ?1"#,
            hash
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await
        )?;
        let option = rows
            .map(|row| Transaction::proto_deserialize(row.raw)).transpose();
        option
    }

    pub async fn query_observation_entry(&self, hash: &Hash) -> RgResult<Option<TransactionEntry>> {
        let hash = hash.safe_bytes()?;
        let rows =  DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw, time FROM observation WHERE hash = ?1"#,
            hash
        ).fetch_optional(&mut *self.ctx.pool().await?).await)?;
        let option = rows
            .map(|row| Transaction::proto_deserialize(row.raw).map(|t| TransactionEntry{
                transaction: Some(t),
                time: row.time as u64
            })).transpose();
        option
    }

    pub async fn recent_observation(&self, limit: Option<i64>) -> Result<Vec<Transaction>, ErrorInfo> {
        let limit = limit.unwrap_or(10);
        let rows =  DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM observation ORDER BY height DESC LIMIT ?1"#,
            limit
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?;
        rows.iter().map(|r| Transaction::proto_deserialize_ref(&r.raw)).collect()
    }

    pub async fn insert_observation_and_edges(
        &self,
        tx: &structs::Transaction,
    ) -> Result<i64, ErrorInfo> {
        let time = tx.time()?;
        let hash = tx.hash_or();
        let height = tx.height()?;
        let observation = tx.observation()?;
        let utxo_id = observation.parent_id.safe_get_msg("Missing parent id")?;
        let option1 = tx.input_of(utxo_id);
        let input = option1.safe_get_msg("Missing input")?;
        let option = input.proof.get(0);
        let input_proof = option.safe_get_msg("Missing input proof")?;
        let pk = input_proof.public_key.safe_get_msg("Missing public key")?;

        let res = self.insert_observation(
            tx, time.clone(), &hash, height, pk
        ).await?;
        // TODO: we can actually use the sql derive class here since the class instance is the same
        // as the table -- modify the table slightly to match so we don't have to store the binary
        for proof in observation.build_observation_proofs(&hash, input_proof) {
            let mut edge = ObservationEdge::default();
            edge.time = time.clone();
            edge.observation_proof = Some(proof);
            self.insert_observation_edge(&edge).await?;
        }
        Ok(res)
    }

    pub async fn query_time_observation_edge(&self, start: i64, end: i64) -> Result<Vec<ObservationEdge>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT edge, time FROM observation_edge WHERE time >= ?1 AND time <= ?2"#,
            start,
            end
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let proof = ObservationProof::proto_deserialize(row.edge)?;
            let time = row.time.clone();
            let mut edge = ObservationEdge::default();
            edge.observation_proof = Some(proof);
            edge.time = time;
            res.push(edge)
        }
        Ok(res)
    }

    pub async fn select_observation_edge(&self, observed_hash: &Hash) -> Result<Vec<ObservationProof>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let bytes = observed_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT edge FROM observation_edge WHERE observed_hash = ?1"#,
            bytes
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            res.push(ObservationProof::proto_deserialize(row.edge)?)
        }
        Ok(res)
    }


    pub async fn insert_observation_edge(&self, observation_edge: &ObservationEdge) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let proof = observation_edge.observation_proof.safe_get()?;
        let merkle = proof.merkle_proof.clone();
        let root = merkle.clone().and_then(|m| m.root.clone()).safe_get()?.safe_bytes()?;
        let edge = proof.proto_serialize();
        let leaf_hash = merkle.and_then(|m| m.leaf.clone()).safe_bytes()?;
        let obs_hash = proof.observation_hash.safe_bytes()?;
        let observed_hash = proof.metadata.safe_get()?.observed_hash.safe_bytes()?;
        let time = util::current_time_millis();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO observation_edge
            (root, leaf_hash, observation_hash, observed_hash, edge, time) VALUES
            (?1, ?2, ?3, ?4, ?5, ?6)"#,
            root,
            leaf_hash,
            obs_hash,
            observed_hash,
            edge,
            time
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }
}
