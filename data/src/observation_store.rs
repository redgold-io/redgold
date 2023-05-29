use sqlx::query::Map;
use sqlx::{Error, Sqlite};
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use redgold_schema::structs::{Address, ErrorInfo, FixedUtxoId, Hash, Observation, ObservationEdge, ObservationEntry, ObservationProof, PeerData, PublicKey, Transaction, UtxoEntry};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants, util, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct ObservationStore {
    pub ctx: DataStoreContext
}

use crate::schema::json_or;

impl ObservationStore {

    pub async fn select_latest_observation(&self, peer_key: PublicKey) -> Result<Option<Observation>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = peer_key.bytes()?;
        let rows = sqlx::query!(
            r#"SELECT raw_observation FROM observation WHERE public_key = ?1 ORDER BY height DESC LIMIT 1"#,
            bytes
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_observation;
            if let Some(o) = option1 {
                let deser = Observation::proto_deserialize(o)?;
                res.push(deser);
            }
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

    pub async fn insert_observation(&self, observation: &Observation, time: i64) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let root =  observation.merkle_root.safe_bytes()?;
        let ser = observation.proto_serialize();
        let public_key = observation.proof.safe_get()?.public_key_bytes()?.clone();
        let height = observation.height;
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO observation
            (root, raw_observation, public_key, time, height) VALUES
            (?1, ?2, ?3, ?4, ?5)"#,
            root,
            ser,
            public_key,
            time,
            height
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn query_time_observation(&self, start_time: i64, end_time: i64) -> Result<Vec<ObservationEntry>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw_observation, time FROM observation WHERE time >= ?1 AND time <= ?2"#,
            start_time,
            end_time
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_observation;
            if let Some(o) = option1 {
                let deser = Observation::proto_deserialize(o)?;
                let time = row.time.safe_get()?.clone();
                let mut entry = ObservationEntry::default();
                entry.observation = Some(deser);
                entry.time = time as u64;
                res.push(entry);
            }
        }
        Ok(res)
    }

    pub async fn insert_observation_and_edges(&self, observation: &Observation, time: i64) -> Result<i64, ErrorInfo> {
        let res = self.insert_observation(observation, time).await?;
        // TODO: we can actually use the sql derive class here since the class instance is the same
        // as the table -- modify the table slightly to match so we don't have to store the binary
        for proof in observation.build_observation_proofs() {
            let mut edge = ObservationEdge::default();
            edge.time = time;
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
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let proof = ObservationProof::proto_deserialize(row.edge)?;
            let time = row.time.safe_get()?.clone();
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
            .fetch_all(&mut pool)
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
        let root =  merkle.clone().and_then(|m| m.root.clone()).safe_get()?.safe_bytes()?;
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
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }


}
