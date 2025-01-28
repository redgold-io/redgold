use jsonrpc_core::{Id, MethodCall, Params, Version};
use monero_rpc::RpcAuthentication;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use diqwest::WithDigestAuth;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJson;
use serde::{Deserialize, Serialize};

// https://docs.getmonero.org/rpc-library/wallet-rpc/#introduction
pub struct MoneroWalletRpcMultisigClient {
    addr: String,
    http_client: reqwest::Client,
    rpc_auth: RpcAuthentication,
}

#[ignore]
#[tokio::test]
async fn verify_multisig_test() {
    let rpc = MoneroWalletRpcMultisigClient::new(
        "http://server:28088".to_string(),
        Some("username:password".to_string())
    ).unwrap();
}


#[derive(Debug, serde::Deserialize, Clone, Serialize)]
struct MoneroRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct MoneroRpcResponse<T> {
    result: Option<T>,
    error: Option<MoneroRpcError>,
    id: String,
    jsonrpc: String,
}


impl MoneroWalletRpcMultisigClient {

    pub fn new(addr: String, auth_str: Option<String>) -> RgResult<Self> {
        let http_client = reqwest::Client::new();
        let rpc_auth = if let Some(a) = auth_str {
            let user = a.split(":").next().unwrap_or("").to_string();
            let pass = a.split(":").last().unwrap_or("").to_string();
            let auth = RpcAuthentication::Credentials { username: user, password: pass };
            auth
        } else {
            RpcAuthentication::None
        };
        Ok(Self {
            addr,
            http_client,
            rpc_auth,
        })

    }


    /// Prepare the wallet for multisig by generating the initial multisig keys
    pub async fn prepare_multisig(&self) -> RgResult<String> {
        // Call the prepare_multisig method with empty parameters
        let params = Params::None;
        let response = self.json_rpc_call("prepare_multisig", params).await?;

        // Extract the multisig_info string from the response
        let multisig_info = response.get("multisig_info")
            .and_then(|v| v.as_str())
            .ok_msg("Failed to extract multisig_info from response")?
            .to_string();

        Ok(multisig_info)
    }

    /// Make a wallet multisig by importing peers' multisig strings
    ///
    /// # Arguments
    /// * `multisig_info` - List of multisig strings from peers
    /// * `threshold` - Amount of signatures needed to sign a transfer
    /// * `password` - Wallet password
    ///
    /// # Returns
    /// * `MakeMultisigResult` containing the multisig wallet address and additional multisig info (if any)
    pub async fn make_multisig(
        &mut self,
        multisig_info: Vec<String>,
        threshold: u32,
        password: String,
    ) -> RgResult<MakeMultisigResult> {

        println!("Make multisig threshold with {} {}", multisig_info.len(), threshold);
        let mut params: Map<String, Value> = Default::default();
        params.insert("multisig_info".to_string(), json!(multisig_info));
        params.insert("threshold".to_string(), json!(threshold));
        params.insert("password".to_string(), json!(password));

        let response = self.json_rpc_call("make_multisig", Params::Map(params)).await?;

        let address = response.get("address")
            .and_then(|v| v.as_str())
            .ok_msg("Failed to extract address from response")?
            .to_string();

        let multisig_info = response.get("multisig_info")
            .and_then(|v| v.as_str())
            .ok_msg("Failed to extract multisig_info from response")?
            .to_string();

        Ok(MakeMultisigResult {
            address,
            multisig_info,
        })
    }


    async fn json_rpc_call(
        &self,
        method: &'static str,
        params: Params,
        ) -> RgResult<Value> {
            let client = self.http_client.clone();
            let uri = format!("{}/json_rpc", &self.addr);

            let method_call = MethodCall {
                jsonrpc: Some(Version::V2),
                method: method.to_string(),
                params: params.into(),
                id: Id::Str(Uuid::new_v4().to_string()),
            };
        println!("JSON RPC Request: {:?}", method_call);


        let req = client.post(&uri).json(&method_call);

            // Get the raw response text first
            let response_text = if let RpcAuthentication::Credentials { username, password } = &self.rpc_auth {
                req.send_with_digest_auth(username, password)
                    .await
                    .error_info("Failed to send request")?
                    .text()
                    .await
                    .error_info("Failed to get response text")?
            } else {
                req.send()
                    .await
                    .error_info("send err")?
                    .text()
                    .await
                    .error_info("Failed to get response text")?
            };

        println!("JSON RPC Response text: {}", response_text.clone());
            // Parse the response
            let response: MoneroRpcResponse<Value> = serde_json::from_str(&response_text)
                .error_info("Failed to parse JSON response")?;

            // Handle potential RPC error
            if let Some(error) = response.error {
                error.json_or().to_error()?;
            }
            Ok(response.result.ok_msg("No result in response")?)

    }

    /// Exchange multisig keys with other participants
    ///
    /// # Arguments
    /// * `multisig_info` - Vector of multisig key information strings from other participants
    /// * `password` - Wallet password
    ///
    /// # Returns
    /// * Result containing a struct with the multisig address and whether the wallet is ready
    pub async fn exchange_multisig_keys(
        &mut self,
        multisig_info: Vec<String>,
        password: String,
        force_update_use_with_caution: Option<bool>,
    ) -> RgResult<ExchangeMultisigKeysResult> {
        let mut params: Map<String, Value> = Default::default();
        params.insert("multisig_info".to_string(), json!(multisig_info));
        params.insert("password".to_string(), json!(password));
        if let Some(force) = force_update_use_with_caution {
            params.insert("force_update_use_with_caution".to_string(), json!(force));
        } else {
            params.insert("force_update_use_with_caution".to_string(), json!(true));
        }
        let response = self.json_rpc_call("exchange_multisig_keys", Params::Map(params)).await?;

        let address = response.get("address")
            .and_then(|v| v.as_str())
            .ok_msg("Failed to extract address from response")?
            .to_string();

        let multisig_info = response.get("multisig_info")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_msg("Failed to extract multisig info from response")?;

        Ok(ExchangeMultisigKeysResult {
            address,
            multisig_info
        })
    }
    /// Export multisig info for other participants
    /// This must be called by all participants to get information needed to transfer
    ///
    /// # Returns
    /// * Result containing the multisig info string
    pub async fn export_multisig_info(&mut self) -> RgResult<String> {
        let response = self.json_rpc_call("export_multisig_info", Params::None).await?;

        let info = response.get("info")
            .and_then(|v| v.as_str())
            .ok_msg("Failed to extract multisig info from response")?
            .to_string();

        Ok(info)
    }

    /// Import multisig info from other participants
    /// This must be called with info from all other participants before creating transactions
    ///
    /// # Arguments
    /// * `info` - Vector of multisig info strings from other participants
    ///
    /// # Returns
    /// * Result containing number of outputs signed with those key images
    pub async fn import_multisig_info(&mut self, info: Vec<String>) -> RgResult<u64> {
        let mut params: Map<String, Value> = Default::default();
        params.insert("info".to_string(), json!(info));

        let response = self.json_rpc_call("import_multisig_info", Params::Map(params)).await?;

        let n_outputs = response.get("n_outputs")
            .and_then(|v| v.as_u64())
            .ok_msg("Failed to extract n_outputs from response")?;

        Ok(n_outputs)
    }

    /// Sign a multisig transaction
    ///
    /// # Arguments
    /// * `multisig_txset` - The multisig transaction set to sign
    ///
    /// # Returns
    /// * Result containing the signed transaction info
    pub async fn sign_multisig(
        &mut self,
        multisig_txset: String,
    ) -> RgResult<SignedMultisigTxset> {
        let mut params: Map<String, Value> = Default::default();
        params.insert("tx_data_hex".to_string(), json!(multisig_txset));

        let response = self.json_rpc_call("sign_multisig", Params::Map(params)).await?;

        Ok(SignedMultisigTxset {
            tx_data_hex: response.get("tx_data_hex")
                .and_then(|v| v.as_str())
                .ok_msg("Failed to extract tx_data_hex")?
                .to_string(),
            tx_hash_list: response.get("tx_hash_list")
                .and_then(|v| v.as_array())
                .ok_msg("Failed to extract tx_hash_list")?
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect(),
        })
    }
    /// Finalize a multisig wallet (required extra step for N-1/N wallets)
    ///
    /// # Arguments
    /// * `multisig_info` - List of multisig strings from peers
    /// * `password` - Wallet password
    ///
    /// # Returns
    /// * Result containing the finalized multisig wallet address
    pub async fn finalize_multisig(
        &mut self,
        multisig_info: Vec<String>,
        password: String,
    ) -> RgResult<String> {
        let mut params: Map<String, Value> = Default::default();
        params.insert("multisig_info".to_string(), json!(multisig_info));
        params.insert("password".to_string(), json!(password));

        let response = self.json_rpc_call("finalize_multisig", Params::Map(params)).await?;

        let address = response.get("address")
            .and_then(|v| v.as_str())
            .ok_msg("Failed to extract address from response")?
            .to_string();

        Ok(address)
    }

    /// Submit a signed multisig transaction to the network
    ///
    /// # Arguments
    /// * `tx_data_hex` - The signed transaction data in hexadecimal format
    ///
    /// # Returns
    /// * Result containing the transaction hashes
    pub async fn submit_multisig(
        &mut self,
        tx_data_hex: String,
    ) -> RgResult<Vec<String>> {
        let mut params: Map<String, Value> = Default::default();
        params.insert("tx_data_hex".to_string(), json!(tx_data_hex));

        let response = self.json_rpc_call("submit_multisig", Params::Map(params)).await?;

        let tx_hash_list = response.get("tx_hash_list")
            .and_then(|v| v.as_array())
            .ok_msg("Failed to extract tx_hash_list")?
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();

        Ok(tx_hash_list)
    }
    /*
    Check if a wallet is a multisig one.

    Alias: None.

    Inputs: None.

    Outputs:

    multisig - boolean; States if the wallet is multisig
    ready - boolean;
    threshold - unsigned int; Amount of signature needed to sign a transfer.
    total - unsigned int; Total amount of signature in the multisig wallet.
    Example for a non-multisig wallet:
     */
    pub async fn is_multisig(&mut self) -> RgResult<IsMultisigResponse> {
        let response = self.json_rpc_call("is_multisig", Params::None).await?;

        let multisig = response.get("multisig")
            .and_then(|v| v.as_bool())
            .ok_msg("Failed to extract multisig from response")?;

        let ready = response.get("ready")
            .and_then(|v| v.as_bool())
            .ok_msg("Failed to extract ready from response")?;

        let threshold = response.get("threshold")
            .and_then(|v| v.as_u64())
            .ok_msg("Failed to extract threshold from response")?;

        let total = response.get("total")
            .and_then(|v| v.as_u64())
            .ok_msg("Failed to extract total from response")?;

        Ok(IsMultisigResponse {
            multisig,
            ready,
            threshold: threshold as u32,
            total: total as u32,
        })

    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsMultisigResponse {
    pub multisig: bool,
    pub ready: bool,
    pub threshold: u32,
    pub total: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferDestination {
    pub amount: u64,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareMultisigResult {
    pub multisig_txset: String,
    pub unsigned_txset: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedMultisigTxset {
    pub tx_data_hex: String,
    pub tx_hash_list: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExchangeMultisigKeysResult {
    pub address: String,
    pub multisig_info: String
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MakeMultisigResult {
    pub address: String,
    pub multisig_info: String,
}