use sqlx::Sqlite;
use redgold_schema::structs::{ErrorInfo, Transaction};
use redgold_schema::RgResult;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use crate::DataStoreContext;
use crate::transaction_store::TransactionStore;

impl TransactionStore {

    pub async fn insert_transaction_raw(
        &self,
        tx: &Transaction,
        time: i64,
        rejection_reason: Option<ErrorInfo>,
        sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<i64, ErrorInfo> {

        if let Some(r) = rejection_reason {
            return self.insert_rejected_transaction(
                tx, r, sqlite_tx).await;
        }


        let hash = tx.hash_or().vec();
        let transaction_proto = tx.proto_serialize();
        let time = tx.time()?.clone();
        let signable_hash = tx.signable_hash_or().vec();
        // Do we want to filter out fee transactions here? Or remainders? Maybe
        let first_input_address = tx.first_input_address().map(|a| a.proto_serialize());
        let first_output_address = tx.first_output_address_non_input_or_fee().map(|a| a.proto_serialize());
        let transaction_type = tx.transaction_type()? as i32;
        let total_amount = tx.total_output_amount();
        let first_output_amount = tx.first_output_amount_i64();
        let fee_amount = tx.fee_amount();
        let remainder_amount = tx.remainder_amount();
        let contract_type = tx.first_contract_type().map(|x| x as i32);
        /*
           hash    BLOB PRIMARY KEY NOT NULL,
          transaction_proto       BLOB NOT NULL,
          time       INTEGER NOT NULL,
          signable_hash       BLOB NOT NULL,
          first_input_address       BLOB,
          first_output_address       BLOB,
          transaction_type       BLOB NOT NULL,
          total_amount       INTEGER NOT NULL,
          first_output_amount       INTEGER,
          fee_amount       INTEGER,
          remainder_amount       INTEGER,
          contract_type       INTEGER,
         */
        let is_test = tx.is_test();
        let is_swap = tx.is_swap();
        let is_metadata = tx.is_metadata();
        let is_request = tx.is_request();
        let is_deploy = tx.is_deploy();
        let is_liquidity = tx.is_stake();
        /*

          is_test INTEGER NOT NULL,
          is_swap INTEGER NOT NULL,
          is_metadata INTEGER NOT NULL,
          is_request INTEGER NOT NULL,
          is_deploy INTEGER NOT NULL,
          is_liquidity INTEGER NOT NULL
               */
        // let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query_as!(
                    sqlx::sqlite::SqliteQueryResult,
            r#"
        INSERT OR REPLACE INTO transactions
        (hash, transaction_proto, time, signable_hash, first_input_address, first_output_address,
        transaction_type, total_amount, first_output_amount, fee_amount, remainder_amount, contract_type,
        is_test, is_swap, is_metadata, is_request, is_deploy, is_liquidity)
        VALUES (
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
        )"#,
        hash, transaction_proto, time, signable_hash, first_input_address, first_output_address,
        transaction_type, total_amount, first_output_amount, fee_amount, remainder_amount, contract_type,
        is_test, is_swap, is_metadata, is_request, is_deploy, is_liquidity
        )
            .execute(&mut **sqlite_tx)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn insert_rejected_transaction(&self, tx: &Transaction, rejection_reason: ErrorInfo, sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>) -> RgResult<i64> {
        let hash = tx.hash_or().vec();
        let ser = tx.proto_serialize();
        let rejection_ser = rejection_reason.proto_serialize();
        let time = tx.time()?.clone();
        /*
                sqlite_tx: &mut sqlx::Transaction<'_, Sqlite>
    ) -> Result<i64, ErrorInfo> {

        let rejection_ser = rejection_reason.clone().map(|x| x.json_or());
        let accepted = rejection_reason.is_none();
        let is_test = tx.is_test();
        let hash_vec = tx.hash_bytes()?;
        let ser = tx.proto_serialize();

        // let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query_as!(
                    sqlx::sqlite::SqliteQueryResult,
         */
        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO rejected_transactions
        (hash, transaction_proto, time, rejection_reason) VALUES (?1, ?2, ?3, ?4)"#,
           hash, ser, time, rejection_ser
        )
            .execute(&mut *self.ctx.pool().await?)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }



}