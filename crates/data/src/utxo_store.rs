use crate::schema::SafeOption;
use crate::DataStoreContext;
use itertools::Itertools;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, Hash, PublicKey, SupportedCurrency, UtxoEntry, UtxoId};
use redgold_schema::{structs, RgResult};
use sqlx::Sqlite;

#[derive(Clone)]
pub struct UtxoStore {
    pub ctx: DataStoreContext
}

impl UtxoStore {


    pub async fn count_distinct_address_utxo(
        &self
    ) -> Result<i64, ErrorInfo> {
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT COUNT(DISTINCT(address)) as count FROM utxo"#
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?.count as i64)
    }


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
        self.utxo_id_valid_opt(utxo, None).await
    }


    pub async fn utxo_id_valid_opt(
        &self,
        utxo: &UtxoId,
        tx_opt: Option<&mut sqlx::Transaction<'_, Sqlite>>
    ) -> Result<bool, ErrorInfo> {
        let b = utxo.transaction_hash.safe_get()?.vec();
        // TODO: Select present
        let rows = sqlx::query!(
            r#"SELECT output_index FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            b,
            utxo.output_index
        );
        let fetched_rows = DataStoreContext::map_err_sqlx(match tx_opt {
            Some(mut tx) => {
                // If a transaction is provided, use it directly.
                rows.fetch_optional(&mut **tx).await
            }
            None => {
                let mut pool = self.ctx.pool().await?;
                rows.fetch_optional(&mut *pool).await
            }
        })?;
        Ok(fetched_rows.is_some())
    }


    pub async fn utxo_children(
        &self,
        utxo_id: &UtxoId,
    ) -> RgResult<Vec<(Hash, i64)>> {
        self.utxo_children_pool_opt(utxo_id, None).await
    }

    pub async fn utxo_children_pool_opt(
        &self,
        utxo_id: &UtxoId,
        tx_opt: Option<&mut sqlx::Transaction<'_, Sqlite>>
    ) -> RgResult<Vec<(Hash, i64)>> {
        let bytes = utxo_id.transaction_hash.safe_get()?.vec();
        let output_index = utxo_id.output_index;

        let rows = sqlx::query!(
            r#"SELECT child_transaction_hash, child_input_index FROM transaction_edge
            WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        );

        // Decide whether to use the provided transaction or create a new pool.
        let fetched_rows = DataStoreContext::map_err_sqlx(match tx_opt {
            Some(mut tx) => {
                // If a transaction is provided, use it directly.
                rows.fetch_all(&mut **tx).await
            }
            None => {
                let mut pool = self.ctx.pool().await?; // Assuming pool() returns a Result<Pool, Error>
                rows.fetch_all(&mut *pool).await
            }
        })?;
        fetched_rows
            .into_iter()
            .map(|o| Hash::new_from_proto(o.child_transaction_hash.clone()).map(|h| (h, o.child_input_index)))
            .collect()
    }


    pub async fn utxo_child(
        &self,
        utxo_id: &UtxoId,
    ) -> RgResult<Option<(Hash, i64)>> {
        let bytes = utxo_id.transaction_hash.safe_get()?.vec();
        let output_index = utxo_id.output_index;
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT child_transaction_hash, child_input_index FROM transaction_edge
            WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await)?
            .map(|o| Hash::new_from_proto(o.child_transaction_hash).map(|h| (h, o.child_input_index)))
            .transpose()
    }

    pub async fn delete_utxo(
        &self,
        fixed_utxo_id: &UtxoId,
        sqlite_tx: Option<&mut sqlx::Transaction<'_, Sqlite>>
    ) -> Result<u64, ErrorInfo> {

        let transaction_hash = fixed_utxo_id.transaction_hash.safe_get()?;
        let output_index = fixed_utxo_id.output_index.clone();
        let bytes = transaction_hash.vec();
        let rows = sqlx::query!(
            r#"DELETE FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        );

        // Decide whether to use the provided transaction or create a new pool.
        let fetched_rows = DataStoreContext::map_err_sqlx(match sqlite_tx {
            Some(mut tx) => {
                // If a transaction is provided, use it directly.
                rows.execute(&mut **tx).await
            }
            None => {
                let mut pool = self.ctx.pool().await?; // Assuming pool() returns a Result<Pool, Error>
                rows.execute(&mut *pool).await
            }
        })?;
        let rows = fetched_rows.rows_affected();

        // gauge!("redgold.utxo.total").decrement(rows as f64);
        Ok(rows)
    }

    pub async fn utxo_for_address(
        &self,
        address: &Address
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {

        let bytes = address.vec();
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
        let bytes = transaction_hash.vec();
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
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT DISTINCT transaction_hash FROM utxo WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        ).fetch_all(&mut *self.ctx.pool().await?).await)?
            .into_iter()
            .map(|row| Hash::new_from_proto(row.transaction_hash)).collect()
    }

    pub async fn insert_utxo(
        &self,
        utxo_entry: &UtxoEntry,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<i64, ErrorInfo> {
        // let mut pool = self.ctx.pool().await?;
        let id = utxo_entry.utxo_id.safe_get_msg("missing utxo id")?;
        let hash = id.transaction_hash.safe_get()?.vec();
        let output_index = id.output_index;
        let output = utxo_entry.output.safe_get_msg("UTxo entry insert missing output")?;
        let amount = output.opt_amount().clone();
        let output_ser = output.proto_serialize();
        let raw = utxo_entry.proto_serialize();
        let addr = utxo_entry.address()?;
        let address = addr.vec();

        let has_code = output.validate_deploy_code().is_ok();
        let qry = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO utxo (transaction_hash, output_index,
        address, output, time, amount, raw, has_code) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            hash,
            output_index,
            address,
            output_ser,
            utxo_entry.time,
            amount,
            raw,
            has_code
        );
        let rows = qry
            .execute(&mut **sqlite_tx)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        // gauge!("redgold.utxo.total").increment(1.0);
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn query_utxo_output_index(
        &self,
        transaction_hash: &Hash,
    ) -> Result<Vec<i32>, ErrorInfo> {
        let bytes = transaction_hash.vec();
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT output_index FROM utxo WHERE transaction_hash = ?1"#,
            bytes
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?.into_iter().map(|row| row.output_index as i32).collect_vec())
    }

    pub async fn get_balance_for_addresses(&self, addresses: Vec<Address>) -> RgResult<CurrencyAmount> {
        let mut bal = CurrencyAmount::zero(SupportedCurrency::Redgold);
        for addr in addresses {
            let utxos = self.utxo_for_address(&addr).await?;
            for utxo in utxos {
                let output = utxo.output.safe_get_msg("missing output")?;
                let amount = output.opt_amount_typed_ref();
                if let Some(amount) = amount {
                    if amount.is_rdg() {
                        bal = bal + amount.clone();
                    }
                }
            }
        }
        Ok(bal)
    }

    pub async fn get_balance_for_public_key(&self, key: &PublicKey) -> RgResult<CurrencyAmount> {
        let addresses = key.to_all_addresses()?;
        self.get_balance_for_addresses(addresses).await
    }



}



// Debug stuff below
impl UtxoStore {

    pub async fn utxo_all_debug(
        &self
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw FROM utxo"#,
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            res.push(UtxoEntry::proto_deserialize(row.raw)?)
        }
        Ok(res)
    }


    pub async fn utxo_filter_time(
        &self,
        start: i64,
        end: i64
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw FROM utxo WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            res.push(UtxoEntry::proto_deserialize(row.raw)?)
        }
        Ok(res)
    }

}