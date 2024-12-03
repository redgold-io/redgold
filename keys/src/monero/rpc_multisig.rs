use redgold_schema::RgResult;
use crate::monero::rpc_core::MoneroRpcWrapper;
//
// impl MoneroRpcWrapper {
//
//     /// Prepare the wallet for multisig by generating the initial multisig keys
//     pub async fn prepare_multisig(&mut self) -> RgResult<String> {
//         let result = self.client.clone().daemon_rpc()
//             .await
//             .error_info("Failed to prepare multisig")?;
//         Ok(result.multisig_info)
//     }
//
//
// }