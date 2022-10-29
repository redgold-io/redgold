// use crate::data::data_store::DataStore;
// use crate::schema::ProtoHashable;
// use crate::schema::structs::{Block, ErrorInfo};
//
// pub trait BlockStore {
//     async fn query_last_block(&self) -> Result<Option<Block>, ErrorInfo>;
// }
//
// impl BlockStore for DataStore {
//     async fn query_last_block(&self) -> Result<Option<Block>, ErrorInfo> {
//         let mut pool = self.pool().await?;
//         let rows = sqlx::query!("SELECT raw FROM block ORDER BY height DESC LIMIT 1")
//             .fetch_one(&mut pool)
//             .await;
//         let rows_m = DataStore::map_err_sqlx(rows)?;
//         match rows_m.raw {
//             None => Ok(None),
//             Some(b) => Ok(Some(Block::proto_deserialize(b)?)),
//         }
//     }
//
//     pub async fn insert_block(&self, block: &Block) -> Result<i64, ErrorInfo> {
//         let mut pool = self.pool().await?;
//
//         let hash = block.hash_bytes()?;
//         let height = block.height as i64;
//         let raw = block.proto_serialize();
//         let time = block.time()? as i64;
//
//         let rows = sqlx::query!(
//             r#"
//         INSERT INTO block ( hash, height, raw, time )
//         VALUES ( ?1, ?2, ?3, ?4)
//                 "#,
//             hash,
//             height,
//             raw,
//             time
//         )
//         .execute(&mut pool)
//         .await;
//         let rows_m = DataStore::map_err_sqlx(rows)?;
//         Ok(rows_m.last_insert_rowid())
//     }
// }
