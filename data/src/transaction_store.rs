use sqlx::query::Map;
use sqlx::{Error, Sqlite};
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use redgold_schema::structs::{Address, ErrorInfo, FixedUtxoId, Hash, PeerData, Transaction, UtxoEntry};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants, WithMetadataHashable};
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
            r#"SELECT raw_transaction FROM transactions WHERE hash = ?1"#,
            transaction_hash
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_transaction;
            if let Some(o) = option1 {
                let deser = Transaction::proto_deserialize(o)?;
                res.push(deser);
            }
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

    pub async fn query_accepted_transaction(
        &self,
        transaction_hash: &Hash,
    ) -> Result<Option<Transaction>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;

        let bytes = transaction_hash.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT raw_transaction FROM transactions WHERE hash = ?1 AND rejection_reason IS NULL AND accepted = 1"#,
            bytes
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_transaction;
            if let Some(o) = option1 {
                let deser = Transaction::proto_deserialize(o)?;
                res.push(deser);
            }
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

    pub async fn query_recent_transactions(
        &self, limit: Option<i64>
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let limit = limit.unwrap_or(10);
        let mut pool = self.ctx.pool().await?;

        // : Map<Sqlite, fn(SqliteRow) -> Result<Record, Error>, SqliteArguments>
        let map = sqlx::query!(
            r#"SELECT raw_transaction FROM transactions WHERE rejection_reason IS NULL AND accepted = 1
            ORDER BY time DESC LIMIT ?1"#,
            limit
        );
        let rows = map.fetch_all(&mut pool).await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_transaction;
            if let Some(o) = option1 {
                let deser = Transaction::proto_deserialize(o)?;
                res.push(deser);
            }
        }
        Ok(res)
    }

    pub async fn count_total_accepted_transactions(
        &self
    ) -> Result<i64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM transactions WHERE rejection_reason IS NULL AND accepted = 1"#
        )
            .fetch_all(&mut pool)
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
            r#"SELECT raw_transaction, rejection_reason FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_transaction;
            let rejection = row.rejection_reason;
            if let Some(o) = option1 {
                let deser = Transaction::proto_deserialize(o)?;
                let mut rej = None;
                if let Some(r) = rejection {
                    let rr = ErrorInfo::proto_deserialize(r)?;
                    rej = Some(rr);

                }
                res.push((deser, rej ))
            }
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
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
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.output_index;
            if let Some(o) = option1 {
                res.push(o as i32);
            }
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
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.output_index;
            if let Some(o) = option1 {
                res.push(o as i64);
            }
        }
        Ok(!res.is_empty())
    }

    pub async fn delete_utxo(
        &self,
        fixed_utxo_id: &FixedUtxoId
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
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.rows_affected())
    }


    pub async fn insert_utxo(
        &self,
        utxo_entry: &UtxoEntry
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let hash = utxo_entry.transaction_hash.clone();
        let output_index = utxo_entry.output_index;
        let output = utxo_entry.output.safe_get_msg("UTxo entry insert missing output")?.proto_serialize();
        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO utxo (transaction_hash, output_index, address, output, time) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            hash,
            output_index,
            utxo_entry.address,
            output,
            utxo_entry.time
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }
    //
    // pub async fn insert_transaction_edge(
    //     &self,
    //     utxo_entry: &UtxoEntry,
    //     child_transaction_hash: &Hash,
    //     child_input_index: i64
    // ) -> Result<i64, ErrorInfo> {
    //     let mut pool = self.ctx.pool().await?;
    //     let hash = utxo_entry.transaction_hash.clone();
    //     let child_hash = child_transaction_hash.safe_bytes()?;
    //     let output_index = utxo_entry.output_index;
    //     let rows = sqlx::query!(
    //         r#"
    //     INSERT OR REPLACE INTO transaction_edge
    //     (transaction_hash, output_index, address, child_transaction_hash, child_input_index, time)
    //     VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
    //         hash,
    //         output_index,
    //         utxo_entry.address,
    //         child_hash,
    //         child_input_index,
    //         utxo_entry.time
    //     )
    //         .execute(&mut pool)
    //         .await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     Ok(rows_m.last_insert_rowid())
    // }
    //
    // pub async fn query_transaction_edge_get_children_of(
    //     &self,
    //     transaction_hash: &Hash,
    //     output_index: i64,
    // ) -> Result<Vec<(Hash, i64)>, ErrorInfo> {
    //
    //     let mut pool = self.ctx.pool().await?;
    //     let bytes = transaction_hash.safe_bytes()?;
    //     let rows = sqlx::query!(
    //         r#"SELECT child_transaction_hash, child_input_index FROM transaction_edge
    //         WHERE transaction_hash = ?1 AND output_index = ?2"#,
    //         bytes,
    //         output_index
    //     )
    //         .fetch_all(&mut pool)
    //         .await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     let mut res = vec![];
    //     for row in rows_m {
    //         let option1 = row.child_transaction_hash;
    //         let option2 = row.child_input_index;
    //         if let Some(o) = option1 {
    //             let h = Hash::from_bytes_mh(o);
    //             if let Some(o2) = option2 {
    //                 res.push((h, o2 as i64));
    //             }
    //         }
    //     }
    //     Ok(res)
    // }


    pub async fn insert_transaction_raw(
        &self,
        tx: &Transaction,
        time: i64,
        accepted: bool,
        rejection_reason: Option<ErrorInfo>,
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rejection_ser = rejection_reason.map(|x| json_or(&x));
        let hash_vec = tx.hash_bytes()?;
        let ser = tx.proto_serialize();
        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO transactions
        (hash, raw_transaction, time, rejection_reason, accepted) VALUES (?1, ?2, ?3, ?4, ?5)"#,
           hash_vec, ser, time, rejection_ser, accepted
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn insert_transaction(
        &self,
        tx: &Transaction,
        time: i64,
        accepted: bool,
        rejection_reason: Option<ErrorInfo>,
    ) -> Result<i64, ErrorInfo> {
        let i = self.insert_transaction_raw(tx, time, accepted, rejection_reason).await?;
        for entry in UtxoEntry::from_transaction(tx, time as i64) {
            self.insert_utxo(&entry).await?;
        }
        return Ok(i);
    }
}

#[test]
fn debug() {

}