use redgold_schema::structs::{Address, ErrorInfo, Hash, PeerData, Transaction};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct TransactionStore {
    pub ctx: DataStoreContext
}


impl TransactionStore {



    pub async fn query_transaction_hex(
        &self,
        hex: String,
    ) -> Result<Option<Transaction>, ErrorInfo> {
        let vec = from_hex(hex)?;
        self.query_transaction(&vec).await
    }

    pub async fn query_transaction(
        &self,
        transaction_hash: &Vec<u8>,
    ) -> Result<Option<Transaction>, ErrorInfo> {

        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT raw_transaction FROM transactions WHERE hash = ?1"#,
            transaction_hash
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let option1 = row.raw_transaction;
            if let Some(o) = option1 {
                let deser = Transaction::proto_deserialize(o)?;
                res.push(deser);
            }
        }
        let option = res.get(0).map(|x| x.clone());
        Ok(option)
    }

}

#[test]
fn debug() {

}