// use redgold_keys::address_external::ToEthereumAddress;
// use redgold_schema::RgResult;
// use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey};
// use redgold_keys::eth::example::EthHistoricalClient;
//
// pub trait PublicKeyToAllBalances {
//     fn get_external_balances(&self, network_environment: NetworkEnvironment) -> RgResult<Vec<CurrencyAmount>>;
// }
//
// // TODO: Later
// impl PublicKeyToAllBalances for PublicKey {
//     fn get_external_balances(&self, network_environment: NetworkEnvironment) -> RgResult<Vec<CurrencyAmount>> {
//         let mut res = vec![];
//
//         if let Some(Ok(eth)) = EthHistoricalClient::new(&network_environment) {
//             let eth_address = self.to_ethereum_address()?;
//             let bi = eth.get_balance(&eth_address).await?;
//
//         }
//
//         Ok(res)
//
//     }
// }