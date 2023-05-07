use std::result;
use std::sync::Arc;
use sqlx::pool::PoolConnection;
use sqlx::{Sqlite, SqlitePool};
use redgold_schema::error_message;
use redgold_schema::structs::ErrorInfo;

pub use redgold_schema as schema;
pub mod address_block;
pub mod peer;
pub mod config;
pub mod servers;
pub mod transaction_store;
pub mod mp_store;

#[derive(Clone)]
pub struct DataStoreContext {
    pub connection_path: String,
    pub pool: Arc<SqlitePool>,
}

impl DataStoreContext {

    pub async fn pool(&self) -> Result<PoolConnection<Sqlite>, ErrorInfo> {
        DataStoreContext::map_err_sqlx(self.pool.acquire().await)
    }

    pub fn map_err_sqlx<A>(error: Result<A, sqlx::Error>) -> Result<A, ErrorInfo> {
        error.map_err(|e| error_message(schema::structs::Error::InternalDatabaseError, e.to_string()))
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
