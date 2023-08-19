use std::collections::HashSet;
use metrics::{decrement_gauge, increment_gauge};
use redgold_keys::TestConstants;
use redgold_schema::structs::{Address, ErrorInfo, FixedUtxoId, Hash, Output, Transaction, TransactionEntry, UtxoEntry};
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
        &self, address: &Address, has_code: bool
    ) -> RgResult<Option<UtxoEntry>> {
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM utxo WHERE has_code = ?1"#,
            has_code
        ).fetch_optional(&mut *self.ctx.pool().await?).await)
            .and_then(|r|
                r.map(|r| structs::UtxoEntry::proto_deserialize(r.raw)).transpose()
            )
    }
}