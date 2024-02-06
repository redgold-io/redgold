use std::collections::HashSet;
use itertools::Itertools;
use metrics::gauge;
use redgold_keys::TestConstants;
use redgold_schema::structs::{Address, ErrorInfo, UtxoId, Hash, Output, Transaction, TransactionEntry, UtxoEntry};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, RgResult, SafeBytesAccess, structs, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct TransactionStore {
    pub ctx: DataStoreContext
}

use crate::schema::json_or;

impl TransactionStore {

    pub async fn query_transaction_hex(
        &self,
        hex: String,
    ) -> Result<Option<Transaction>, ErrorInfo> {
        let vec = from_hex(hex)?;
        self.query_transaction(&vec).await
    }

    pub async fn query_transaction(
        &self,
        transaction_hash: &Vec<u8>,
    ) -> Result<Option<Transaction>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw FROM transactions WHERE hash = ?1"#,
            transaction_hash
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.raw)?;
            res.push(deser);
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

    pub async fn query_time_transaction(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<TransactionEntry>> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw, time FROM transactions WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.raw)?;
            let te = TransactionEntry{
                time: row.time as u64,
                transaction: Some(deser),
            };
            res.push(te);
        }
        Ok(res)
    }

    pub async fn query_accepted_transaction(
        &self,
        transaction_hash: &Hash,
    ) -> Result<Option<Transaction>, ErrorInfo> {

        let bytes = transaction_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT raw FROM transactions WHERE hash = ?1 AND rejection_reason IS NULL AND accepted = 1"#,
            bytes
        )
            .fetch_all(&mut *self.ctx.pool().await?)
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

    pub async fn query_recent_transactions(
        &self, limit: Option<i64>,
        is_test: Option<bool>
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let limit = limit.unwrap_or(10);
        let mut pool = self.ctx.pool().await?;

        // : Map<Sqlite, fn(SqliteRow) -> Result<Record, Error>, SqliteArguments>
        let map = match is_test {
            None => {
                DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM transactions WHERE rejection_reason IS NULL AND accepted = 1
            ORDER BY time DESC LIMIT ?1"#,
            limit
            ).fetch_all(&mut *pool).await)?.iter().map(|row| Transaction::proto_deserialize(row.raw.clone()))
                    .collect::<RgResult<Vec<Transaction>>>()
            }
            Some(b) => {
                DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM transactions WHERE rejection_reason IS NULL
            AND accepted = 1 AND is_test=?2
            ORDER BY time DESC LIMIT ?1"#,
            limit, b).fetch_all(&mut *pool).await)?.iter().map(|row| Transaction::proto_deserialize(row.raw.clone()))
                    .collect::<RgResult<Vec<Transaction>>>()
            }
        };
        map
    }
    pub async fn recent_transaction_hashes(
        &self,
        limit: Option<i64>,
        min_time: Option<i64>
    ) -> Result<Vec<Hash>, ErrorInfo> {
        let limit = limit.unwrap_or(10);
        let min_time = min_time.unwrap_or(0);
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT hash FROM transactions
            WHERE rejection_reason IS NULL AND accepted = 1 AND time > ?1
            ORDER BY time DESC
            LIMIT ?2"#,
            min_time,
            limit
        ).fetch_all(&mut *self.ctx.pool().await?).await)?.iter()
            .map(|t| Hash::new(t.hash.clone()))
            .collect_vec())
    }

    pub async fn count_total_accepted_transactions(
        &self
    ) -> Result<i64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM transactions WHERE rejection_reason IS NULL AND accepted = 1"#
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

    pub async fn count_total_transactions(
        &self
    ) -> Result<i64, ErrorInfo> {
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT COUNT(*) as count FROM transactions"#
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?.count as i64)
    }

    pub async fn count_total_utxos(
        &self
    ) -> Result<i64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM utxo"#
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

    //
    // pub async fn count_total_accepted_transactions(
    //     &self
    // ) -> Result<i64, ErrorInfo> {
    //
    //     let qry = sqlx::query!(
    //         r#"SELECT COUNT(*) as count FROM transactions WHERE rejection_reason IS NULL AND accepted = 1"#
    //     );
    //     let vec = self.ctx.run_query(
    //         qry, |r | Ok(r.count)
    //     ).await?;
    //     Ok(vec.get(0).safe_get()?.clone())
    // }

    pub async fn query_maybe_transaction(
        &self,
        transaction_hash: &Hash,
    ) -> Result<Option<(Transaction, Option<ErrorInfo>)>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;

        let bytes = transaction_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT raw, rejection_reason FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw;
            let rejection = row.rejection_reason;
            let deser = Transaction::proto_deserialize(option1)?;
            let mut rej = None;
            if let Some(r) = rejection {
                let rr = ErrorInfo::proto_deserialize(r)?;
                rej = Some(rr);
            }
            res.push((deser, rej ))
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

    pub async fn transaction_known(
        &self,
        transaction_hash: &Hash,
    ) -> RgResult<bool> {
        let bytes = transaction_hash.safe_bytes()?;
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT count(raw) as count FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?
            .count > 0)
    }


    pub async fn query_utxo_output_index(
        &self,
        transaction_hash: &Hash,
    ) -> Result<Vec<i32>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = transaction_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT output_index FROM utxo WHERE transaction_hash = ?1"#,
            bytes
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let o = row.output_index;
            res.push(o as i32);
        }
        Ok(res)
    }

    pub async fn query_utxo_id_valid(
        &self,
        transaction_hash: &Hash,
        output_index: i64,
    ) -> Result<bool, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = transaction_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT output_index FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let o = row.output_index;
            res.push(o as i64);
        }
        Ok(!res.is_empty())
    }

    pub async fn get_balance(&self, address: &Address) -> RgResult<Option<i64>> {
        self.query_utxo_address(address).await.map(|utxos| {
            let mut balance = 0;
            for utxo in &utxos {
                if let Some(o) = &utxo.output {
                    if let Some(a) = o.opt_amount() {
                        balance += a;
                    }
                }
            }
            if balance > 0 {
                Some(balance)
            } else {
                None
            }
        })
    }

    pub async fn utxo_for_addresses(&self, addresses: &Vec<Address>) -> RgResult<Vec<UtxoEntry>> {
        let mut res = vec![];
        for address in addresses {
            let utxos = self.query_utxo_address(address).await?;
            res.extend(utxos);
        }
        Ok(res)
    }

    pub async fn query_utxo_address(
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

    pub async fn utxo_scroll(
        &self, limit: i64, offset: i64
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw FROM utxo LIMIT ?1 OFFSET ?2"#,
            limit,
            offset
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

    pub async fn delete_utxo(
        &self,
        fixed_utxo_id: &UtxoId
    ) -> Result<u64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let transaction_hash = fixed_utxo_id.transaction_hash.safe_get()?;
        let output_index = fixed_utxo_id.output_index.clone();
        let bytes = transaction_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"DELETE FROM utxo WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        gauge!("redgold.utxo.total").increment(1.0);
        Ok(rows_m.rows_affected())
    }



// TODO: Add productId to utxo amount


    pub async fn insert_transaction_edge(
        &self,
        utxo_id: &UtxoId,
        address: &Address,
        child_transaction_hash: &Hash,
        child_input_index: i64,
        time: i64
    ) -> Result<i64, ErrorInfo> {

        let hash = utxo_id.transaction_hash.safe_get_msg("No transaction hash on utxo_id")?.vec();
        let child_hash = child_transaction_hash.safe_bytes()?;
        let output_index = utxo_id.output_index;
        let address = address.address.safe_bytes()?;
        let rows = DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"
        INSERT OR REPLACE INTO transaction_edge
        (transaction_hash, output_index, address, child_transaction_hash, child_input_index, time)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            hash,
            output_index,
            address,
            child_hash,
            child_input_index,
            time
        )
            .execute(&mut *self.ctx.pool().await?)
            .await)?;
        Ok(rows.last_insert_rowid())
    }

    pub async fn utxo_used(
        &self,
        utxo_id: &UtxoId,
    ) -> Result<Option<(Hash, i64)>, ErrorInfo> {
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


    pub async fn insert_transaction_raw(
        &self,
        tx: &Transaction,
        time: i64,
        accepted: bool,
        rejection_reason: Option<ErrorInfo>,
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rejection_ser = rejection_reason.map(|x| json_or(&x));
        let is_test = tx.is_test();
        let hash_vec = tx.hash_bytes()?;
        let ser = tx.proto_serialize();

        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO transactions
        (hash, raw, time, rejection_reason, accepted, is_test) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
           hash_vec, ser, time, rejection_ser, accepted, is_test
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }


    pub async fn insert_address_transaction_single(
        &self,
        address: &Address,
        tx_hash: &Hash,
        time: i64,
        incoming: bool
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let hash_vec = tx_hash.safe_bytes()?;
        let address_vec = address.address.safe_bytes()?;
        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO address_transaction
        (address, tx_hash, time, incoming) VALUES (?1, ?2, ?3, ?4)"#,
           address_vec, hash_vec, time, incoming
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn get_all_tx_for_address(
        &self,
        address: &Address,
        limit: i64,
        offset: i64
    ) -> Result<Vec<Transaction>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = address.address.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT tx_hash FROM address_transaction WHERE address = ?1 ORDER BY time DESC LIMIT ?2 OFFSET ?3"#,
            bytes, limit, offset
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let tx_hash: Vec<u8> = row.tx_hash.clone();
            let tx_hash = Hash::new(tx_hash);
            // Suppress failed transactions from listing, maybe add a flag to show them
            if let Some((tx, None)) = self.query_maybe_transaction(&tx_hash).await? {
                res.push(tx)
            }
        }
        Ok(res)
    }

    pub async fn get_filter_tx_for_address(
        &self,
        address: &Address,
        limit: i64,
        offset: i64,
        incoming: bool
    ) -> Result<Vec<Transaction>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = address.address.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT tx_hash FROM address_transaction WHERE address = ?1 AND incoming=?4 ORDER BY time DESC LIMIT ?2 OFFSET ?3"#,
            bytes, limit, offset, incoming
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let tx_hash: Vec<u8> = row.tx_hash.clone();
            let tx_hash = Hash::new(tx_hash);
            // Suppress failed transactions from listing, maybe add a flag to show them
            if let Some((tx, None)) = self.query_maybe_transaction(&tx_hash).await? {
                res.push(tx)
            }
        }
        Ok(res)
    }

    pub async fn get_count_filter_tx_for_address(
        &self,
        address: &Address,
        incoming: bool
    ) -> Result<i64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = address.address.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(tx_hash) as count FROM address_transaction WHERE address = ?1 AND incoming=?2"#,
            bytes, incoming
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        for row in rows_m {
            let count = row.count;
            return Ok(count as i64);
        }
        Ok(0)
    }

    pub async fn insert_address_transaction(&self, tx: &Transaction) -> RgResult<()> {
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
            self.insert_address_transaction_single(&address, &hash, time.clone(), true).await?;
        }
        for address in addr_outgoing {
            self.insert_address_transaction_single(&address, &hash, time.clone(), false).await?;
        }

        Ok(())

    }

    pub async fn insert_transaction(
        &self,
        tx: &Transaction,
        time: i64,
        accepted: bool,
        rejection_reason: Option<ErrorInfo>,
        update_utxo: bool
    ) -> Result<i64, ErrorInfo> {
        let i = self.insert_transaction_raw(tx, time.clone(), accepted, rejection_reason).await?;
        let vec = UtxoEntry::from_transaction(tx, time.clone() as i64);
        if update_utxo {
            for entry in vec {
                self.insert_utxo(&entry).await?;
            }
        }
        for (i, x)  in tx.inputs.iter().enumerate() {
            if let Some(utxo) = &x.utxo_id {
                self.insert_transaction_edge(
                    utxo,
                    &x.address()?,
                    &tx.hash_or(),
                    i as i64,
                    time.clone()).await?;
            }
        }
        self.insert_address_transaction(tx).await?;
        gauge!("redgold.transaction.accepted.total").increment(1.0);
        return Ok(i);
    }
    //
    // // This doesn't seem to work correctly, not returning proper xor
    // pub async fn xor_transaction_order(&self, hash: &Hash) -> RgResult<Vec<(Vec<u8>, Vec<u8>)>> {
    //     let mut pool = self.ctx.pool().await?;
    //     let hash_vec = hash.safe_bytes()?;
    //     let rows = sqlx::query!(
    //         r#"SELECT hash, COALESCE( (hash | ?1) - (hash & ?1), X'0000') as xor FROM transactions"#,
    //        hash_vec //
    //     )
    //         .fetch_all(&mut *pool)
    //         .await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     let mut res = vec![];
    //     for row in rows_m {
    //         let xor: Vec<u8> = row.xor;
    //         res.push((row.hash.expect(""), xor));
    //     }
    //     Ok(res)
    // }


}


// Move to UTXO store.
impl TransactionStore {
    pub async fn insert_utxo(
        &self,
        utxo_entry: &UtxoEntry
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let id = utxo_entry.utxo_id.safe_get_msg("missing utxo id")?;
        let hash = id.transaction_hash.safe_bytes()?;
        let output_index = id.output_index;
        let output = utxo_entry.output.safe_get_msg("UTxo entry insert missing output")?;
        let amount = output.opt_amount().clone();
        let output_ser = output.proto_serialize();
        let raw = utxo_entry.proto_serialize();
        let addr = utxo_entry.address()?;
        let address = addr.address.safe_bytes()?;

        let has_code = output.validate_deploy_code().is_ok();
        let rows = sqlx::query!(
            r#"
        INSERT INTO utxo (transaction_hash, output_index,
        address, output, time, amount, raw, has_code) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            hash,
            output_index,
            address,
            output_ser,
            utxo_entry.time,
            amount,
            raw,
            has_code
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        gauge!("redgold.utxo.total").increment(1.0);
        Ok(rows_m.last_insert_rowid())
    }

}


#[test]
fn debug() {
}