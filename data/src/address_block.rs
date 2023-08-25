use std::collections::HashMap;
use std::result;
use sqlx::{Executor, Row, Sqlite};
use redgold_schema::{error_message, ProtoSerde, RgResult, SafeBytesAccess};
use redgold_schema::structs::{Address, AddressBlock, Block, Error, ErrorInfo, Output};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct AddressBlockStore {
    pub ctx: DataStoreContext
}

#[derive(Clone)]
pub struct AddressHistoricalBalance {
    pub address: Address,
    pub balance: i64,
    pub height: i64
}

impl AddressBlockStore {

    pub async fn all_address_balance_by_height(
        &self,
        height: i64,
    ) -> Result<Vec<AddressHistoricalBalance>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT address, balance FROM address_block WHERE height = ?1"#,
            height
        )
            .fetch_all( &mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            res.push(AddressHistoricalBalance{
                address: Address::from_bytes(row.address.safe_get_msg("Row missing address")?.clone())?,
                balance: row.balance.safe_get_msg("Row missing balance")?.clone(),
                height: height.clone()
            });
        }
        Ok(res)
    }

    pub async fn query_last_block(&self) -> RgResult<Option<Block>> {
        let mut pool = self.ctx.pool().await?;

        // TODO change this to a fetch all in case nothing is returned on initialization.
        let rows = sqlx::query!("SELECT raw FROM block ORDER BY height DESC LIMIT 1")
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        match rows_m {
            None => Ok(None),
            Some(b) => Ok(Some(Block::proto_deserialize(b.raw)?)),
        }
    }

    pub async fn query_last_balance_address(
        &self,
        address: &Vec<u8>,
    ) -> result::Result<Option<i64>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT balance FROM address_block WHERE address = ?1 ORDER BY height DESC LIMIT 1"#,
            address
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        for row in rows_m {
            return Ok(row.balance)
        }
        Ok(None)
    }

    pub async fn insert_address_block(
        &self,
        address_block: AddressBlock,
    ) -> result::Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let rows = sqlx::query!(
            r#"
        INSERT INTO address_block ( address, balance, height, hash )
        VALUES ( ?1, ?2, ?3, ?4)
                "#,
            address_block.address,
            address_block.balance,
            address_block.height,
            address_block.hash
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn query_address_balance_by_height(
        &self,
        address: Address,
        height: i64,
    ) -> result::Result<Option<i64>, ErrorInfo> {
        // select address balance
        let address_bytes = address.address.safe_bytes()?;
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT balance FROM address_block WHERE address = ?1 AND height <= ?2 ORDER BY height DESC LIMIT 1"#,
            address_bytes,
            height
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        for row in rows_m {
            return Ok(row.balance);
        }
        Ok(None)
    }

    // TODO: Just resolve the hash to a height, way easier.
    pub async fn query_block_hash_height(
        &self,
        hash: Option<Vec<u8>>,
        height: Option<i64>,
    ) -> result::Result<Option<Block>, ErrorInfo> {
        if hash.is_none() && height.is_none() {
            return Err(error_message(
                Error::MissingField,
                "Block hash and height both empty",
            ));
        }
        let mut pool = self.ctx.pool().await?;
        let mut clause_str: String = "WHERE ".to_string();
        if hash.is_some() {
            clause_str += "hash = ?1";
            if height.is_some() {
                clause_str += " AND height = ?2";
            }
        } else {
            clause_str += "height = ?1";
        }

        let query_str = format!("SELECT raw FROM block {} LIMIT 2", clause_str);
        let mut query = sqlx::query(&query_str);
        if let Some(h) = hash.clone() {
            query = query.bind(h);
            if let Some(hh) = height {
                query = query.bind(hh);
            }
        } else {
            query = query.bind(height.expect("Height shouldn't be empty with earlier validator"));
        }

        let rows = query.fetch_all(&mut *pool).await;

        let rows_m: Vec<_> = DataStoreContext::map_err_sqlx(rows)?;
        if rows_m.is_empty() {
            return Ok(None);
        }
        if rows_m.len() > 1 {
            return Err(error_message(
                Error::DataStoreInternalCorruption,
                format!(
                    "More than 1 block returned on query for hash {} height {}",
                    hash.map(|h| hex::encode(h)).unwrap_or("none".to_string()),
                    height.map(|h| h.to_string()).unwrap_or("none".to_string())
                ),
            ));
        }
        for row in rows_m {
            let raw: Vec<u8> = DataStoreContext::map_err_sqlx(row.try_get("raw"))?;
            return Ok(Some(Block::proto_deserialize(raw)?));
        }
        return Ok(None);
    }

    // TODO: Move ba
    //
    // pub async fn insert_block_update_historicals(&self, block: &Block) -> Result<(), ErrorInfo> {
    //     let vec = block.transactions.clone();
    //     let height = block.height.clone();
    //     let block_hash = block.hash_bytes()?;
    //
    //
    //     let mut deltas: HashMap<Vec<u8>, i64> = HashMap::new();
    //
    //     for tx in vec {
    //         for o in tx.outputs {
    //             for a in &o.address.clone() {
    //                 for d in &o.data {
    //                     for amount in d.amount {
    //                         let address_bytes = a.address.safe_bytes()?;
    //                         let a1 = address_bytes.clone();
    //                         let maybe_amount = deltas.get(&a1);
    //                         deltas.insert(
    //                             address_bytes,
    //                             maybe_amount.map(|m| m.clone() + amount).unwrap_or(amount),
    //                         );
    //                     }
    //                 }
    //             }
    //         }
    //         for i in tx.inputs {
    //             // TODO: There's a better way to persist these than querying transaction
    //             let tx_hash = i.transaction_hash.safe_bytes()?;
    //             let tx_input =
    //                 DataStoreContext::map_err(self.query_transaction(&tx_hash))?
    //                     .expect("change later");
    //             let prev_output: Output = tx_input
    //                 .outputs
    //                 .get(i.output_index as usize)
    //                 .expect("change later")
    //                 .clone();
    //             for a in prev_output.clone().address {
    //                 for d in &prev_output.data {
    //                     for amount in d.amount {
    //                         let address_bytes = a.address.safe_bytes()?;
    //                         let a1 = address_bytes.clone();
    //                         let maybe_amount = deltas.get(&a1);
    //                         deltas.insert(
    //                             address_bytes,
    //                             maybe_amount
    //                                 .map(|m| m.clone() - amount)
    //                                 .unwrap_or(-1 * amount),
    //                         );
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //
    //     for (k, v) in deltas.iter() {
    //         let vec1 = k.clone();
    //         let res = self.query_last_balance_address(&vec1).await?;
    //         let new_balance = match res {
    //             None => v.clone(),
    //             Some(rr) => v + rr,
    //         };
    //         self.insert_address_block(AddressBlock {
    //             address: k.clone(),
    //             balance: new_balance,
    //             height,
    //             hash: block_hash.clone(),
    //         })
    //             .await?;
    //     }
    //     self.insert_block(&block).await?;
    //     Ok(())
    // }

    pub async fn insert_block(&self, block: &Block) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let hash = block.hash_bytes()?;
        let height = block.height as i64;
        let raw = block.proto_serialize();
        let time = block.time()? as i64;

        let rows = sqlx::query!(
            r#"
        INSERT INTO block ( hash, height, raw, time )
        VALUES ( ?1, ?2, ?3, ?4)
                "#,
            hash,
            height,
            raw,
            time
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

}