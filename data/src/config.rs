use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use redgold_schema::structs::{Address, ErrorInfo, Hash};
use redgold_schema::{ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants};
use crate::DataStoreContext;
use crate::schema::SafeOption;
use crate::schema::json;
use serde::Serialize;

#[derive(Clone)]
pub struct ConfigStore {
    pub ctx: DataStoreContext
}

impl ConfigStore {

    pub async fn store_latest_self_observation() -> Result<(), ErrorInfo> {
        unimplemented!()
    }

    pub async fn insert_update(
        &self,
        key: String,
        value: String
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO config ( key_name, value_data ) VALUES ( ?1, ?2)
                "#,
            key, value
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }
    //
    // pub async fn insert_update_bytes(
    //     &self,
    //     key: String,
    //     value: Vec<u8>
    // ) -> Result<i64, ErrorInfo> {
    //     let mut pool = self.ctx.pool().await?;
    //
    //     let rows = sqlx::query!(
    //         r#"
    //     INSERT OR REPLACE INTO config ( key_name, value_bytes ) VALUES ( ?1, ?2)
    //             "#,
    //         key, value
    //     )
    //         .execute(&mut pool)
    //         .await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     Ok(rows_m.last_insert_rowid())
    // }

    pub async fn insert_update_json<T: Serialize, S: Into<String>>(
        &self,
        key: S,
        value: T
    ) -> Result<i64, ErrorInfo> {
        let key: String = key.into();
        let mut pool = self.ctx.pool().await?;
        let val = json(&value)?;
        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO config ( key_name, value_data ) VALUES ( ?1, ?2)
                "#,
            key, val
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn select_config(
        &self,
        key: String
    ) -> Result<Option<String>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let rows = sqlx::query("SELECT value_data FROM config WHERE key_name = ?1")
            .bind(key)
            .map(|x: SqliteRow| {
                let value_data: &str = x.try_get("value_data")?;
                Ok(value_data.to_string())
            })
            .fetch_optional(&mut pool)
            .await;
        let rows2 = DataStoreContext::map_err_sqlx(rows)?;

        match rows2 {
            None => {Ok(None)}
            Some(r) => {
                let rows_m = DataStoreContext::map_err_sqlx(r)?;
                Ok(Some(rows_m))
            }
        }

        // TODO: Debug null issue?
        // let rows = sqlx::query!(
        //     r#"
        // SELECT (key_name, value_data) FROM config WHERE key_name = ?1
        //         "#,
        //     key
        // )
        //     .fetch_one(&mut pool)
        //     .await;
        //
        // let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        // match rows_m.value_data {
        //     None => Ok(None),
        //     Some(b) => Ok(Some(b)),
        // }
    }

}
