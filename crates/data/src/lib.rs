use std::sync::Arc;
use sqlx::{Pool, Sqlite, SqlitePool, PoolConnection};
use redgold_schema::structs::ErrorInfo;
use crate::error_convert::ResultErrorInfoExt;

pub mod data_store;
pub mod error_convert;
pub mod transaction_store;
pub mod utxo_store;
pub mod transaction_observability;
pub mod price_time;
pub mod state_store;
pub mod mp_store;
pub mod servers;
pub mod peer;
pub mod config;
pub mod observation_store;
pub mod parquet_export;
pub mod parquet_min_index;
pub mod parquet_full_index;
pub mod transaction_insert;
pub mod address_transaction;

pub use redgold_schema as schema;

#[derive(Clone)]
pub struct DataStoreContext {
    pub file_path: String,
    pub connection_path: String,
    pub pool: Arc<SqlitePool>,
}

impl DataStoreContext {
    pub async fn pool(&self) -> Result<PoolConnection<Sqlite>, ErrorInfo> {
        self.pool.acquire().await.map_err_to_info()
    }

    pub async fn run_migrations(&self) -> Result<(), ErrorInfo> {
        sqlx::migrate!("./migrations")
            .run(&*self.pool)
            .await
            .map_err_to_info()
    }

    #[deprecated(
        since = "0.1.0",
        note = "Use .map_err_to_info() from ResultErrorInfoExt trait instead"
    )]
    pub fn map_err_sqlx<A>(error: Result<A, sqlx::Error>) -> Result<A, ErrorInfo> {
        error.map_err_to_info()
    }
}
        since = "0.1.0",

        note = "Use .map_err_to_info() from ResultErrorInfoExt trait instead"

    )]

    pub fn map_err_sqlx<A>(error: Result<A, sqlx::Error>) -> Result<A, ErrorInfo> {

        error.map_err_to_info()

    }

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
