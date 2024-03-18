use std::collections::HashSet;
use itertools::Itertools;
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

}