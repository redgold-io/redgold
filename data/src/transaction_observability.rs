use redgold_schema::{RgResult, SafeOption};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{ErrorInfo, Hash, Transaction};
use redgold_schema::util::times;
use crate::DataStoreContext;
use crate::transaction_store::TransactionStore;

impl TransactionStore {

    pub async fn count_total_accepted_transactions(
        &self
    ) -> Result<i64, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM transactions"#
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

    pub async fn query_rejected_transaction(&self, hash: &Hash) -> RgResult<Option<(Transaction, ErrorInfo)>> {
        let b = hash.proto_serialize();
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT transaction_proto, rejection_reason FROM rejected_transactions WHERE hash = ?1"#,
            b
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await)?.map(|row| Transaction::proto_deserialize_ref(&row.transaction_proto)
            .and_then(|t| ErrorInfo::proto_deserialize(row.rejection_reason).map(|e| (t, e))))
            .transpose()
    }

    pub async fn count_rejected(&self) -> RgResult<i64> {
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT COUNT(*) as count FROM rejected_transactions"#
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?.count as i64)
    }

    pub async fn delete_old_rejected_transaction(
        &self, time_filter: Option<i64>, limit: Option<i64>
    ) -> RgResult<u64> {

        let ct = times::current_time_millis();
        let min_time = ct - (3600*24*7*1000);
        let min_time = time_filter.unwrap_or_else(|| min_time);
        let mut rows_changed = self.delete_rejected_before(min_time).await?;
        let limit = match limit {
            None => {
                let max_count_exceed = 1e5 as i64;
                let count = self.count_rejected().await?;
                if count > max_count_exceed {
                    Some(count - max_count_exceed)
                } else {
                    None
                }

            }
            Some(l) => { Some(l)}
        };
        if let Some(l) = limit {
            let min_time_limit = DataStoreContext::map_err_sqlx(sqlx::query!(
                r#"SELECT time FROM rejected_transactions ORDER BY time DESC LIMIT 1 OFFSET ?1"#,
                l
            ).fetch_optional(&mut *self.ctx.pool().await?).await)?.map(|row| row.time).unwrap_or(0);
            rows_changed += self.delete_rejected_before(min_time).await?;
        }
        Ok(rows_changed)
    }

    async fn delete_rejected_before(&self, min_time: i64) -> Result<u64, ErrorInfo> {
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"DELETE FROM rejected_transactions WHERE time < ?1"#,
            min_time
        ).execute(&mut *self.ctx.pool().await?).await)?.rows_affected())
    }
}