use crate::DataStoreContext;
use crate::error_convert::ResultErrorInfoExt;
use futures::{StreamExt, TryFutureExt};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, ErrorInfo, Hash, Transaction, UtxoEntry, UtxoId};
use redgold_schema::{RgResult, SafeOption};
use sqlx::Executor;

#[derive(Clone)]
pub struct TransactionStore {
    pub ctx: DataStoreContext
}

impl TransactionStore {
    #[deprecated]
    pub async fn query_time_transaction(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Transaction>> {
        sqlx::query!(
            r#"SELECT transaction_proto FROM transactions WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await
            .map_err_to_info()?
            .into_iter()
            .map(|row| Transaction::proto_deserialize(row.transaction_proto))
            .collect::<RgResult<Vec<Transaction>>>()
    }

    pub async fn accepted_time_tx_hashes(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Hash>> {
        sqlx::query!(
            r#"SELECT hash FROM transactions WHERE time >= ?1 AND time < ?2"#,
            start,
            end
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await
            .map_err_to_info()?
            .into_iter()
            .map(|row| Hash::new_from_proto(row.hash))
            .collect()
    }

    pub async fn query_time_transaction_accepted_ordered(
        &self,
        start: i64,
        end: i64
    ) -> RgResult<Vec<Transaction>> {
        sqlx::query!(
            r#"SELECT transaction_proto, time FROM transactions WHERE time >= ?1 AND time < ?2 ORDER BY time ASC"#,
            start,
            end
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await
            .map_err_to_info()?
            .iter()
            .map(|row| {
                Transaction::proto_deserialize(row.transaction_proto.clone())
            })
            .collect()
    }

    pub async fn query_accepted_transaction(
        &self,
        transaction_hash: &Hash
    ) -> Result<Option<Transaction>, ErrorInfo> {
        let bytes = transaction_hash.proto_serialize();
        sqlx::query!(
            r#"SELECT transaction_proto FROM transactions WHERE hash = ?1"#,
            bytes
        )
            .fetch_optional(&mut *self.ctx.pool().await?)
            .await
            .map_err_to_info()?
            .map(|row| Transaction::proto_deserialize(row.transaction_proto))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn it_works() {
        // Add test implementation
    }
}