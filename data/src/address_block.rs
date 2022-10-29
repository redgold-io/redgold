use redgold_schema::structs::{Address, ErrorInfo};
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
            .fetch_all(&mut pool)
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

}