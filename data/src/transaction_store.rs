use std::collections::HashSet;
use sqlx::query::Map;
use sqlx::{Error, Sqlite};
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use redgold_schema::structs::{Address, ErrorInfo, FixedUtxoId, Hash, Output, PeerData, Transaction, UtxoEntry};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, RgResult, SafeBytesAccess, TestConstants, WithMetadataHashable};
use redgold_schema::transaction::AddressBalance;
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

    pub async fn query_utxo_address(
        &self,
        address: &Address
    ) -> Result<Vec<UtxoEntry>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let bytes = address.address.safe_bytes()?;
        let rows = sqlx::query!(
            r#"SELECT transaction_hash, output_index, address, output, time FROM utxo WHERE address = ?1"#,
            bytes
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            //Address::from_bytes(
            let address = row.address.safe_get_msg("Missing Address")?.clone();
            let transaction_hash = row.transaction_hash.safe_get_msg("Missing transaction hash")?.clone();
            let output_index = row.output_index.safe_get_msg("Missing output index")?.clone();
            let time = row.time.safe_get_msg("Missing time")?.clone();
            let output = Some(
                Output::proto_deserialize(row.output.safe_get_msg("Missing output")?.clone())?
            );
            let entry = UtxoEntry {
                transaction_hash,
                output_index,
                address,
                output,
                time,
            };
            res.push(entry)
        }
        Ok(res)
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
            .execute(&mut pool)
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
            .fetch_all(&mut pool)
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
            .fetch_all(&mut pool)
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
            .fetch_all(&mut pool)
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
    ) -> Result<i64, ErrorInfo> {
        let i = self.insert_transaction_raw(tx, time.clone(), accepted, rejection_reason).await?;
        for entry in UtxoEntry::from_transaction(tx, time.clone() as i64) {
            self.insert_utxo(&entry).await?;
        }
        self.insert_address_transaction(tx).await?;
        return Ok(i);
    }
}

#[test]
fn debug() {

}