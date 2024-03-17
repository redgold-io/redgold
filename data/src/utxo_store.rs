use std::collections::HashSet;
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
}