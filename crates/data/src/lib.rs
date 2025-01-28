#![allow(unused_imports)]
#![allow(dead_code)]

use redgold_schema::error_message;
use redgold_schema::structs::ErrorInfo;
use sqlx::pool::PoolConnection;
use sqlx::{Sqlite, SqlitePool};
use std::sync::Arc;

pub use redgold_schema as schema;
pub mod peer;
pub mod config;
pub mod servers;
pub mod transaction_store;
pub mod mp_store;
pub mod observation_store;
pub mod data_store;
pub mod state_store;
pub mod utxo_store;
pub mod parquet_export;
pub mod parquet_min_index;
pub mod parquet_full_index;
pub mod transaction_insert;
pub mod address_transaction;
pub mod transaction_observability;
mod price_time;

#[derive(Clone)]
pub struct DataStoreContext {
    pub file_path: String,
    pub connection_path: String,
    pub pool: Arc<SqlitePool>,
}

impl DataStoreContext {

    pub async fn pool(&self) -> Result<PoolConnection<Sqlite>, ErrorInfo> {
        DataStoreContext::map_err_sqlx(self.pool.acquire().await)
    }

    pub async fn run_migrations(&self) -> Result<(), ErrorInfo> {
        sqlx::migrate!("./migrations")
            .run(&*self.pool)
            .await
            .map_err(|e| error_message(schema::structs::ErrorCode::InternalDatabaseError, e.to_string()))
    }

    pub fn map_err_sqlx<A>(error: Result<A, sqlx::Error>) -> Result<A, ErrorInfo> {
        error.map_err(|e| error_message(schema::structs::ErrorCode::InternalDatabaseError, e.to_string()))
    }

    // This doesn't seem to work due to the Record type here
    // pub async fn run_query<'a, T: Send + Unpin, J>(
    //     &self,
    //     sqlx_macro_query: Map<'a, Sqlite, fn(SqliteRow) -> Result<T, Error>, SqliteArguments<'a>>,
    //     handle_result: fn(T) -> Result<J, ErrorInfo>
    // )
    // -> Result<Vec<J>, ErrorInfo> {
    //     let mut pool = self.pool().await?;
    //     let rows = sqlx_macro_query.fetch_all(&mut *pool).await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     let mut res = vec![];
    //     for row in rows_m {
    //         res.push(handle_result(row)?);
    //     }
    //     Ok(res)
    // }

}


pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
