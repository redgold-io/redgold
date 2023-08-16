use std::time::Duration;
use redgold_schema::structs::{Address, ContractStateMarker, ErrorInfo, FixedUtxoId, Hash, NodeMetadata, PeerData, PeerId, PeerNodeInfo, PublicKey, StateSelector, Transaction};
use redgold_schema::{ProtoHashable, ProtoSerde, RgResult, SafeBytesAccess, util, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;
use itertools::Itertools;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::TestConstants;
use redgold_schema::EasyJson;
use redgold_schema::structs::PeerIdInfo;

#[derive(Clone)]
pub struct StateStore {
    pub ctx: DataStoreContext
}

impl StateStore {
    pub async fn insert_state(&self,
        state: ContractStateMarker
    ) -> Result<i64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let address = state.address.safe_bytes()?;
        let selector_hash = state.selector.safe_get()?.calculate_hash().safe_bytes()?;
        let state_hash = state.state.safe_get()?.calculate_hash().safe_bytes()?;
        let marker = state.transaction_marker.safe_get()?.bytes.safe_bytes()?;
        let nonce = state.nonce.clone();

        let ser = state.proto_serialize();

        let time = state.time.clone();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO state (
            address, selector_hash,
            state_hash, transaction_marker,
            time, nonce, state
            ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7
            )"#,
            address, selector_hash,
            state_hash, marker,
            time, nonce, ser
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }
    pub async fn query_recent_state(&self,
        address: &Address,
        selector: Option<&StateSelector>,
        limit: Option<i64>
    ) -> Result<Vec<ContractStateMarker>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let addr = address.address.safe_bytes()?;
        let limit = limit.unwrap_or(20);
        let x = if let Some(sel) = selector {
            let h = sel.calculate_hash().safe_bytes()?;
            let rows = sqlx::query!(
            r#"SELECT state FROM state WHERE address = ?1 AND selector_hash = ?3 ORDER BY nonce DESC LIMIT ?2"#,
            addr,
            limit,
            h
            ).fetch_all(&mut *pool)
                .await;
            let rows_m = DataStoreContext::map_err_sqlx(rows)?;
            rows_m.iter().map(|i| i.state.clone()).collect_vec()
        } else {
            let rows = sqlx::query!(
            r#"SELECT state FROM state WHERE address = ?1 ORDER BY nonce DESC LIMIT ?2"#,
            addr,
            limit
            ).fetch_all(&mut *pool)
                .await;
            let rows_m = DataStoreContext::map_err_sqlx(rows)?;
            rows_m.iter().map(|i| i.state.clone()).collect_vec()
        };
        x.iter().map(|i| ContractStateMarker::proto_deserialize(i.clone())).collect()
    }
    pub async fn clean_up(&self,
        address: &Address,
        state_sel: &StateSelector,
        nonce: i64
    ) -> Result<u64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let addr = address.address.safe_bytes()?;
        let sel = state_sel.calculate_hash().safe_bytes()?;
        let rows = sqlx::query!(
            r#"DELETE FROM state WHERE address = ?1 AND selector_hash = ?2 AND nonce < ?3"#,
            addr,
            sel,
            nonce
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.rows_affected())
    }

    // Query by latest key etc -- alternatively just delete by time after exporting csvs to parquet?
    pub async fn clean_up_all(&self) -> RgResult<()> {
        // let mut pool = self.ctx.pool().await?
        // let rows = sqlx::query!(
        //     r#"DELETE FROM state WHERE nonce < ?1"#,
        //     0
        // )
        //     .fetch_all(&mut *pool)
        //     .await;
        Ok(())
    }

}
