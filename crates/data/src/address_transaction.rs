use crate::transaction_store::TransactionStore;
use crate::DataStoreContext;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, ErrorInfo, Hash, Transaction};
use redgold_schema::{RgResult, SafeOption};
use sqlx::Sqlite;
use std::collections::HashSet;

// TODO: Consider migrating to own store?
impl TransactionStore {

    pub async fn insert_address_transaction_single(
        &self,
        address: &Address,
        tx_hash: &Hash,
        time: i64,
        incoming: bool,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<i64, ErrorInfo> {
        let hash_vec = tx_hash.vec();
        let address_vec = address.proto_serialize();
        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO address_transaction
        (address, tx_hash, time, incoming) VALUES (?1, ?2, ?3, ?4)"#,
           address_vec, hash_vec, time, incoming
        )
            .execute(&mut **sqlite_tx)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn insert_address_transaction(&self, tx: &Transaction, sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>) -> RgResult<()> {
        let mut addr_incoming = HashSet::new();
        let mut addr_outgoing = HashSet::new();
        for i in &tx.inputs {
            addr_outgoing.insert(i.address()?);
        }
        for i in &tx.outputs {
            addr_incoming.insert(i.address.safe_get_msg("No address on output for insert_address_transaction")?.clone());
        }
        let hash = tx.hash_or();
        let time = tx.struct_metadata.as_ref().and_then(|s| s.time)
            .safe_get_msg("No time on transaction for insert_address_transaction")?.clone();
        for address in addr_incoming {
            self.insert_address_transaction_single(&address, &hash, time.clone(), true, sqlite_tx).await?;
        }
        for address in addr_outgoing {
            self.insert_address_transaction_single(&address, &hash, time.clone(), false, sqlite_tx).await?;
        }

        Ok(())

    }

}