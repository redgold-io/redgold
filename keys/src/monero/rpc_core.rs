use monero::util::address::Address;
use monero_rpc::monero::PrivateKey;
use monero_rpc::{GenerateFromKeysArgs, GetTransfersCategory, GetTransfersSelector, RpcClient, RpcClientBuilder, TransferHeight, WalletCreation};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::{ErrorInfoContext, RgResult, ShortString};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct MoneroRpcWrapper {
    pub url: String,
    pub network: NetworkEnvironment,
    pub client: RpcClient
}

impl MoneroRpcWrapper {

    pub fn from_config(cfg: &NodeConfig) -> Option<RgResult<Self>> {
        if let Some(rpc) = cfg.rpc_url(SupportedCurrency::Monero) {
            let network = cfg.network.clone();
            let ret = MoneroRpcWrapper::new(rpc, network);
            Some(ret)
        } else {
            None
        }
    }
    pub fn new(url: String, network: NetworkEnvironment) -> RgResult<Self> {

        let client = RpcClientBuilder::new()
            .build(url.clone())
            .map_err(|e| ErrorInfo::new(format!("Failed to create Monero RPC client {}", e.to_string())))?;
        let ret = MoneroRpcWrapper {
            url,
            network,
            client,
        };
        Ok(ret)
    }

    pub async fn register_key(
        &self,
        view_key: String,
        address: String,
        spend_key: Option<String>,
        password: Option<String>,
    ) -> RgResult<WalletCreation> {
        let password = password.unwrap_or("".to_string());
        let filename = address.last_n(12)?;
        let address = Address::from_str(&address)
            .error_info("Invalid Monero address")?;
        let pk = PrivateKey::from_str(&view_key)
            .error_info("Invalid Monero view key")?;
        let client = self.client.clone().wallet();

        let spend_key = match spend_key {
            Some(s) => Some(PrivateKey::from_str(&s)
                .error_info("Invalid Monero spend key")?),
            None => None,
        };

        let response = client.generate_from_keys(
            GenerateFromKeysArgs{
                restore_height: None,
                filename,
                address: address,
                spendkey: spend_key,
                viewkey: pk,
                password,
                autosave_current: None,
            }
        ).await
            .map_err(|e| ErrorInfo::new(format!("Failed to register key {}", e.to_string())))?;

        // .error_info("Failed to register Monero key")?;
        Ok(response)
    }

    pub async fn get_all_transactions(&self) -> RgResult<Vec<ExternalTimedTransaction>> {
        let mut hm = std::collections::HashMap::new();
        hm.insert(GetTransfersCategory::In, true);
        hm.insert(GetTransfersCategory::Out, true);
        // hm.insert(GetTransfersCategory::Pending, true);
        // hm.insert(GetTransfersCategory::Failed, true);
        let res = self.client.clone().wallet().get_transfers(
            GetTransfersSelector {
                category_selector: hm,
                account_index: None,
                subaddr_indices: None,
                block_height_filter: None,
            }
        ).await
            .map_err(|e| ErrorInfo::new(format!("Failed to get all {}", e.to_string())))?;
        let mut results = vec![];
        for (k,v) in res.iter() {
            for vv in v.iter() {
                let mut ett = ExternalTimedTransaction::default();
                ett.tx_id = vv.txid.to_string();
                ett.timestamp = Some(vv.timestamp.timestamp_millis() as u64);
                ett.other_address = vv.address.to_string();
                ett.amount = vv.amount.as_pico();
                ett.currency = SupportedCurrency::Monero;
                ett.block_number = match vv.height {
                    TransferHeight::Confirmed(h) => Some(h.get()),
                    TransferHeight::InPool => None,
                };
                ett.incoming = k == &GetTransfersCategory::In;
                ett.fee = Some(CurrencyAmount::from_currency(vv.fee.as_pico() as i64, SupportedCurrency::Monero));
                results.push(ett);
            }
        }
        Ok(results)
    }

    pub async fn get_balance(&self) -> RgResult<CurrencyAmount> {
        let b = self.client.clone().wallet().get_balance(0, None)
            .await
            .map_err(|e| ErrorInfo::new(format!("Failed to get balance {}", e.to_string())))?;
        Ok(CurrencyAmount::from_currency(b.balance.as_pico() as i64, SupportedCurrency::Monero))
    }



}