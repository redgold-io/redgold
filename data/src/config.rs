use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use redgold_schema::structs::{Address, ErrorInfo, Hash};
use redgold_schema::{ErrorInfoContext, ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants};
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

    pub async fn insert_update_bytes(
        &self,
        key: String,
        value: Vec<u8>
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO config ( key_name, value_bytes ) VALUES ( ?1, ?2)
                "#,
            key, value
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

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

        let rows = sqlx::query(
            "SELECT value_data FROM config WHERE key_name = ?1",
        ).bind(key).fetch_optional(&mut pool).await;

        let rows2 = DataStoreContext::map_err_sqlx(rows)?;
        let ret = if let Some(row) = rows2 {
            let str: String = row.try_get("value_data").error_info("value_data not found")?;
            Some(str)
        } else {
            None
        };
        Ok(ret)
    }

    pub async fn select_config_bytes(
        &self,
        key: String
    ) -> Result<Option<Vec<u8>>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let rows = sqlx::query!(
            r#"SELECT value_bytes FROM config WHERE key_name = ?1"#,
            key
         ).fetch_all(&mut pool).await;

        let rows2 = DataStoreContext::map_err_sqlx(rows)?;
        let x = rows2.get(0).safe_get()?.value_bytes.clone();
        Ok(x) // ooh this is cool
    }

    pub async fn get_proto<T: ProtoSerde, S: Into<String>>(&self, key: S) -> Result<T, ErrorInfo> {
        let option = self.select_config_bytes(key.into()).await?;
        let vec = option.safe_get()?.clone();
        T::proto_deserialize(vec)
    }

    pub async fn store_proto<T: ProtoSerde, S: Into<String>>(&self, key: S, value: T) -> Result<i64, ErrorInfo> {
        self.insert_update_bytes(key.into(), value.proto_serialize()).await
    }

}
