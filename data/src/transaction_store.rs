use futures::{StreamExt, TryFutureExt};
use itertools::Itertools;
use redgold_schema::structs::{Address, ErrorInfo, Hash, Transaction, UtxoEntry, UtxoId};
use redgold_schema::{RgResult, SafeOption};
use crate::DataStoreContext;
use sqlx::Executor;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde}; // Make sure this is at the top of your file

#[derive(Clone)]
pub struct TransactionStore {
    pub ctx: DataStoreContext
}

impl TransactionStore {

}

impl TransactionStore {


    #[deprecated]
    pub async fn query_time_transaction(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Transaction>> {
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT transaction_proto FROM transactions WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?.into_iter().map(|row| Transaction::proto_deserialize(row.transaction_proto))
            .collect::<RgResult<Vec<Transaction>>>()
    }

    pub async fn accepted_time_tx_hashes(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Hash>> {
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT hash FROM transactions WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?.into_iter().map(|row| Hash::new_from_proto(row.hash)).collect()
    }

    pub async fn query_time_transaction_accepted_ordered(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Transaction>> {
        let rows = DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT transaction_proto, time FROM transactions WHERE time >= ?1 AND time < ?2 ORDER BY time ASC"#,
            start,
            end
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?;
        rows.iter()
            .map(|row| {
                Transaction::proto_deserialize(row.transaction_proto.clone())
            }).collect()
    }

    // This query stream really needs to be done all in the same function to deal with ownership
    // issues. If using this in the future, then do it directly in line.
    // pub async fn transaction_accepted_ordered_stream(
    //     &self,
    //     start: i64,
    //     end: i64,
    // ) -> RgResult<impl Stream<Item = RgResult<Transaction>>> {
    //
    //     let stream = sqlx::query!(
    //     r#"SELECT raw FROM transactions WHERE rejection_reason IS NULL AND accepted = 1 ORDER BY time ASC"#,
    //     // start, time >= ?1 AND time < ?2 AND
    //     // end
    // )
    //         .fetch(&mut *self.ctx.pool().await?) // Use the awaited pool directly
    //         .map(|row_result| DataStoreContext::map_err_sqlx(row_result)
    //             .and_then(|row| Transaction::proto_deserialize(row.raw))
    //         );
    //     Ok(stream)
    // }

    pub async fn query_accepted_transaction(
        &self,
        transaction_hash: &Hash,
    ) -> Result<Option<Transaction>, ErrorInfo> {
        let bytes = transaction_hash.proto_serialize();
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT transaction_proto FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await)?.map(|row| Transaction::proto_deserialize(row.transaction_proto)).transpose()
    }

    // #[tracing::instrument()]
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
            r#"SELECT transaction_proto FROM transactions
            ORDER BY time DESC LIMIT ?1"#,
            limit
            ).fetch_all(&mut *pool).await)?.into_iter().map(|row| Transaction::proto_deserialize(row.transaction_proto))
                    .collect::<RgResult<Vec<Transaction>>>()
            }
            Some(b) => {
                DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT transaction_proto FROM transactions WHERE is_test=?2
            ORDER BY time DESC LIMIT ?1"#,
            limit, b).fetch_all(&mut *pool).await)?.into_iter().map(|row| Transaction::proto_deserialize(row.transaction_proto))
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
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT hash FROM transactions
            WHERE time > ?1
            ORDER BY time DESC
            LIMIT ?2"#,
            min_time,
            limit
        ).fetch_all(&mut *self.ctx.pool().await?).await)?.into_iter()
            .map(|t| Hash::new_from_proto(t.hash))
            .collect()
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
    ) -> RgResult<Option<(Transaction, Option<ErrorInfo>)>> {
        let res = match self.query_accepted_transaction(transaction_hash).await? {
            None => {
                self.query_rejected_transaction(transaction_hash).await?.map(|(tx, e)| (tx, Some(e)))
            }
            Some(tx) => {
                Some((tx, None))
            }
        };
        Ok(res)
    }

    pub async fn query_accepted_tx(
        &self,
        transaction_hash: &Hash,
    ) -> RgResult<Option<Transaction>> {
        let bytes = transaction_hash.vec();
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT transaction_proto FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await)?.map(|row| Transaction::proto_deserialize(row.transaction_proto)).transpose()
    }

    pub async fn transaction_known(
        &self,
        transaction_hash: &Hash,
    ) -> RgResult<bool> {
        let bytes = transaction_hash.vec();
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT count(*) as count FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?
            .count > 0)
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
    // pub async fn get_balance_retry(&self, address: &Address) -> RgResult<i64> {
    //     retry_ms!(async {
    //         self.get_balance(address).await.and_then(|r| r.ok_msg("error getting balance with retry"))
    //     })
    // }
    //

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

        let bytes = address.vec();
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT raw FROM utxo WHERE address = ?1"#,
            bytes
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?
            .into_iter().map(|row| UtxoEntry::proto_deserialize(row.raw)).collect()
    }


// TODO: Add productId to utxo amount

    pub async fn utxo_used(
        &self,
        utxo_id: &UtxoId,
    ) -> Result<Option<(Hash, i64)>, ErrorInfo> {
        let tx_hash = utxo_id.transaction_hash.safe_get_msg("Tx hash missing")?;
        let bytes = tx_hash.vec();
        let output_index = utxo_id.output_index;
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT child_transaction_hash, child_input_index FROM transaction_edge
            WHERE transaction_hash = ?1 AND output_index = ?2"#,
            bytes,
            output_index
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await)
            .and_then(|oo|
                oo.map(|o| {
                Hash::new_from_proto(o.child_transaction_hash.clone()).map(|h| (h, o.child_input_index))
            })
                    .transpose())
    }
    // pub async fn get_all_tx_for_address_retries(
    //     &self,
    //     address: &Address,
    //     limit: i64,
    //     offset: i64
    // ) -> Result<Vec<Transaction>, ErrorInfo> {
    //     retry_ms!(async { self.get_all_tx_for_address(address, limit, offset).await.and_then(|r| {
    //         if r.len() == 0 {
    //             Err(ErrorInfo::new("No transactions found in retry"))
    //         } else {
    //             Ok(r)
    //         }
    //     })})
    // }

    pub async fn get_all_tx_for_address(
        &self,
        address: &Address,
        limit: i64,
        offset: i64
    ) -> Result<Vec<Transaction>, ErrorInfo> {

        let bytes = address.vec();
        let rows = DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT tx_hash FROM address_transaction WHERE address = ?1 ORDER BY time DESC LIMIT ?2 OFFSET ?3"#,
            bytes, limit, offset
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?;

        // TODO: Convert to map
        let mut res = vec![];
        for row in rows {
            let tx_hash: Vec<u8> = row.tx_hash.clone();
            let tx_hash = Hash::new_from_proto(tx_hash)?;
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
        let bytes = address.vec();
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
            let tx_hash = Hash::new_from_proto(tx_hash)?;
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
        let bytes = address.vec();
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


#[test]
fn debug() {
}