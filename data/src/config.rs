use sqlx::Row;
use redgold_schema::structs::{DynamicNodeMetadata, ErrorInfo, Transaction};
use redgold_schema::{ErrorInfoContext, RgResult};
use crate::DataStoreContext;
use crate::schema::SafeOption;
use redgold_schema::helpers::easy_json::json;
use serde::{Deserialize, Serialize};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::conf::local_stored_state::LocalStoredState;
use redgold_schema::proto_serde::ProtoSerde;

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
            .execute(&mut *pool)
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
            .execute(&mut *pool)
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
            .execute(&mut *pool)
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
        ).bind(key).fetch_optional(&mut *pool).await;

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
         ).fetch_optional(&mut *pool).await;

        let rows2 = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows2.and_then(|row| row.value_bytes))
    }

    pub async fn get_proto<T: ProtoSerde>(&self, key: impl Into<String>) -> Result<T, ErrorInfo> {
        let option = self.select_config_bytes(key.into()).await?;
        let vec = option.safe_get()?.clone();
        T::proto_deserialize(vec)
    }

    pub async fn get_maybe_proto<T: ProtoSerde>(&self, key: impl Into<String>) -> Result<Option<T>, ErrorInfo> {
        if let Some(b) = self.select_config_bytes(key.into()).await? {
            Ok(Some(T::proto_deserialize(b)?))
        } else {
            Ok(None)
        }
    }

    pub async fn store_proto<T: ProtoSerde, S: Into<String>>(&self, key: S, value: T) -> Result<i64, ErrorInfo> {
        self.insert_update_bytes(key.into(), value.proto_serialize()).await
    }

    pub async fn store_proto_ref<T: ProtoSerde, S: Into<String>>(&self, key: S, value: &T) -> Result<i64, ErrorInfo> {
        self.insert_update_bytes(key.into(), value.proto_serialize()).await
    }

    pub async fn get_json<T: for<'de> Deserialize<'de> + Clone>(&self, key: impl Into<String>) -> RgResult<Option<T>> {
        let option = self.select_config(key.into()).await?;
        if let Some(str) = option {
            let res: T = str.json_from()?;
            Ok(Some(res.clone()))
        } else {
            Ok(None)
        }
    }

    pub async fn get_peer_tx(&self) -> RgResult<Option<Transaction>> {
        self.get_maybe_proto("peer_tx").await
    }

    pub async fn set_peer_tx(&self, tx: &Transaction) -> RgResult<i64> {
        self.store_proto_ref("peer_tx", tx).await
    }

    pub async fn get_node_tx(&self) -> RgResult<Option<Transaction>> {
        self.get_maybe_proto("node_tx").await
    }

    pub async fn set_node_tx(&self, tx: &Transaction) -> RgResult<i64> {
        self.store_proto_ref("node_tx", tx).await
    }

    pub async fn get_dynamic_md(&self) -> RgResult<Option<DynamicNodeMetadata>> {
        self.get_maybe_proto("dynamic_node_metadata").await
    }

    pub async fn set_dynamic_md(&self, tx: &DynamicNodeMetadata) -> RgResult<i64> {
        self.store_proto_ref("dynamic_node_metadata", tx).await
    }

    pub async fn get_stored_state(&self) -> RgResult<LocalStoredState> {
        Ok(self.get_json("local_stored_state").await?.unwrap_or(Default::default()))
    }

    pub async fn update_stored_state(&self, local_stored_state: LocalStoredState) -> RgResult<()> {
        self.insert_update_json("local_stored_state", local_stored_state).await.map(|_| ())
    }

    pub async fn store_genesis(&self, gen: &Transaction) -> RgResult<i64> {
        self.store_proto("genesis", gen.clone()).await
    }

    pub async fn get_genesis(&self) -> RgResult<Option<Transaction>> {
        self.get_maybe_proto("genesis").await
    }


}
