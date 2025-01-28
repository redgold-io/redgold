use crate::DataStoreContext;

#[derive(Clone)]
pub struct ServerStore {
    pub ctx: DataStoreContext
}

//
// // TODO: Update this to handle optional types.
//
// impl ServerStore {
//
//     pub async fn servers(
//         &self
//     ) -> Result<Vec<Server>, ErrorInfo> {
//         let mut pool = self.ctx.pool().await?;
//         let rows = sqlx::query!("SELECT host, username, key_path FROM servers")
//             .fetch_all(&mut *pool)
//             .await;
//         let rows_m = DataStoreContext::map_err_sqlx(rows)?;
//         let mut res: Vec<Server> = vec![];
//         for row in rows_m {
//             res.push(Server{
//                 host: row.host,
//                 username: row.username,
//                 key_path: row.key_path
//             });
//         }
//         Ok(res)
//     }
//
//     pub async fn add_server(
//         &self,
//         server: Server
//     ) -> Result<i64, ErrorInfo> {
//         let mut pool = self.ctx.pool().await?;
//
//         let rows = sqlx::query!(
//             r#"
//         INSERT OR REPLACE INTO servers ( host, username, key_path ) VALUES ( ?1, ?2, ?3)
//                 "#,
//             server.host, server.username, server.key_path
//         )
//             .execute(&mut *pool)
//             .await;
//         let rows_m = DataStoreContext::map_err_sqlx(rows)?;
//         Ok(rows_m.last_insert_rowid())
//     }
//
// }