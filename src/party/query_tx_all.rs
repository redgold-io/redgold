use crate::api::client::rest::RgHttpClient;
use async_trait::async_trait;
use redgold_schema::party::address_event::TransactionWithObservationsAndPrice;
use redgold_schema::structs::Address;
use redgold_schema::{error_info, RgResult};

#[async_trait]
pub trait AllTxObsForAddress {
    async fn get_all_tx_obs_for_address(&self, address: &Address, limit: i64, offset: i64) -> RgResult<Vec<TransactionWithObservationsAndPrice>>;
}

#[async_trait]
impl AllTxObsForAddress for RgHttpClient {
    async fn get_all_tx_obs_for_address(&self, _address: &Address, _limit: i64, _offset: i64) -> RgResult<Vec<TransactionWithObservationsAndPrice>> {

        // self.query_hash(address.render_string())
        // let tx = self.get_all_tx_for_address(address, limit, offset).await?;
        // let mut res = vec![];
        // for t in tx {
        //     let h = t.hash_or();
        //     let obs = self.select_observation_edge(&h).await?;
        //     let txo = TransactionWithObservations {
        //         tx: t,
        //         observations: obs,
        //     };
        //     res.push(txo);
        // }
        // Ok(res)
        Err(error_info("Not implemented"))
    }
}
