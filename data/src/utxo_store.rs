use std::collections::HashSet;
use itertools::Itertools;
use metrics::gauge;
use redgold_keys::TestConstants;
use redgold_schema::structs::{Address, ErrorInfo, UtxoId, Hash, Output, Transaction, TransactionEntry, UtxoEntry};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, RgResult, SafeBytesAccess, structs, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct UtxoStore {
    pub ctx: DataStoreContext
}

use crate::schema::json_or;

impl UtxoStore {

    // Good template example to copy elsewhere.
    pub async fn code_utxo(
        &self, _address: &Address, has_code: bool
    ) -> RgResult<Option<UtxoEntry>> {
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM utxo WHERE has_code = ?1"#,
            has_code
        ).fetch_optional(&mut *self.ctx.pool().await?).await)
            .and_then(|r|
                r.map(|r| structs::UtxoEntry::proto_deserialize(r.raw)).transpose()
            )
    }


    pub async fn utxo_id_valid(
        &self,
        utxo: &UtxoId
    ) -> Result<bool, ErrorInfo> {
        let b = utxo.transaction_hash.safe_bytes()?;
        // TODO: Select present
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT output_index FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            b,
            utxo.output_index
        )
            .fetch_optional(&mut *self.ctx.pool().await?).await)?
            .is_some())
    }


    pub async fn utxo_children(
        &self,
        utxo_id: &UtxoId,
    ) -> RgResult<Vec<(Hash, i64)>> {
        let bytes = utxo_id.transaction_hash.safe_bytes()?;
        let output_index = utxo_id.output_index;
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT child_transaction_hash, child_input_index FROM transaction_edge
            WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?
            .iter().map(|o| (Hash::new(o.child_transaction_hash.clone()), o.child_input_index))
            .collect_vec())
    }


    pub async fn utxo_child(
        &self,
        utxo_id: &UtxoId,
    ) -> RgResult<Option<(Hash, i64)>> {
        let bytes = utxo_id.transaction_hash.safe_bytes()?;
        let output_index = utxo_id.output_index;
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT child_transaction_hash, child_input_index FROM transaction_edge
            WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await)?
            .map(|o| (Hash::new(o.child_transaction_hash), o.child_input_index)))
    }

    pub async fn delete_utxo(
        &self,
        fixed_utxo_id: &UtxoId
    ) -> Result<u64, ErrorInfo> {

        let transaction_hash = fixed_utxo_id.transaction_hash.safe_get()?;
        let output_index = fixed_utxo_id.output_index.clone();
        let bytes = transaction_hash.safe_bytes()?;
        let rows = DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"DELETE FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .execute(&mut *self.ctx.pool().await?)
            .await)?.rows_affected();
        gauge!("redgold.utxo.total").decrement(rows as f64);
        Ok(rows)
    }

    pub async fn utxo_for_address(
        &self,
        address: &Address
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {

        let bytes = address.address.safe_bytes()?;
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM utxo WHERE address = ?1"#,
            bytes
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?
            .iter().map(|row| UtxoEntry::proto_deserialize_ref(&row.raw)).collect()
    }

    // This should really only ever return 1 value, otherwise there's an error
    pub async fn utxo_for_id(
        &self,
        fixed_utxo_id: &UtxoId
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {
        let transaction_hash = fixed_utxo_id.transaction_hash.safe_get()?;
        let output_index = fixed_utxo_id.output_index.clone();
        let bytes = transaction_hash.safe_bytes()?;
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?
            .iter().map(|row| UtxoEntry::proto_deserialize_ref(&row.raw)).collect()
    }


    pub async fn utxo_tx_hashes_time(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Hash>> {
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT DISTINCT transaction_hash FROM utxo WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        ).fetch_all(&mut *self.ctx.pool().await?).await)?
            .into_iter()
            .map(|row| Hash::new(row.transaction_hash)).collect_vec())
    }

}