use crate::examples::example;
use alloy_chains::Chain;
use ethers::addressbook::Address;
use ethers::prelude::U256;
use ethers::utils::hex;
use foundry_block_explorers::account::{GenesisOption, NormalTransaction, Sort, TokenQueryOption, TxListParams};
use foundry_block_explorers::Client;
use num_bigint::{BigInt, Sign};
use num_traits::{FromPrimitive, ToPrimitive};
use redgold_keys::address_external::ToEthereumAddress;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{CurrencyAmount, CurrencyId, ErrorInfo, ExternalTransactionId, NetworkEnvironment, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::{error_info, structs, ErrorInfoContext, RgResult, SafeOption};
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info};
use redgold_keys::address_support::AddressSupport;

pub struct EthHistoricalClient {
    pub client: Client,
}

impl EthHistoricalClient {
    pub fn new(network_environment: &NetworkEnvironment) -> Option<RgResult<EthHistoricalClient>> {
        let key = std::env::var("ETHERSCAN_API_KEY").ok();
        key.map(|k| {
            Self::new_from_key(network_environment, k)
        })
    }

    pub fn new_from_key(network_environment: &NetworkEnvironment, k: String) -> Result<EthHistoricalClient, ErrorInfo> {
        let chain = Self::chain_id(network_environment);
        Client::new(chain, k)
            .error_info("Client initialization failure")
            .map(|c| EthHistoricalClient { client: c })
    }

    // This doesn't appear to be working rn
    pub async fn recommended_fee(&self) -> RgResult<BigInt> {
        let fee = self.client.gas_oracle().await.error_info("gas oracle")?;
        let fee = fee.safe_gas_price;
        let vec = fee.to_be_bytes_vec();
        let bi = BigInt::from_bytes_be(Sign::Plus, &*vec);
        Ok(bi)
    }
    pub async fn recommended_fee_typed(&self) -> RgResult<CurrencyAmount> {
        Ok(CurrencyAmount::from_eth_bigint(self.recommended_fee().await?))
    }


        // pub fn query_contract(&self) {
    //     self.client.
    // }

    pub fn chain_id(network_environment: &NetworkEnvironment) -> Chain {
        let chain = if network_environment.is_main() {
            Chain::mainnet()
        } else {
            Chain::sepolia()
        };
        chain
    }

    pub async fn get_balance(&self, address: &String) -> RgResult<String> {
        let addr = address.parse().error_info("address parse failure")?;
        let metadata = self.client
            .get_ether_balance_single(&addr, None).await.error_info("balance fetch failure")?;
        let bal = metadata.balance;
        Ok(bal)
    }

    pub async fn get_balance_typed(&self, address: &structs::Address) -> RgResult<CurrencyAmount> {
        let address = address.render_string()?;
        let addr = address.parse().error_info("address parse failure")?;
        let metadata = self.client
            .get_ether_balance_single(&addr, None).await.error_info("balance fetch failure")?;
        let bal = metadata.balance;
        Ok(CurrencyAmount::from_eth_bigint_string(bal))
    }

    pub async fn check_balance_changed(&self, addr: &structs::Address, max_retries: Option<usize>, status_info: Option<impl Into<String>>) -> RgResult<()> {
        let status_info = status_info.map(|s| s.into()).unwrap_or("".to_string());
        let max_retries = max_retries.unwrap_or(10);
        let original_eth_balance = self.get_balance_typed(&addr).await?;
        let amount_orig = original_eth_balance.amount_i64_or();
        let mut retries = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            info!("Awaiting ETH check_balance_changed {status_info}");
            let new_balance = self.get_balance_typed(&addr).await?;
            let new_amount = new_balance.amount_i64_or();
            if new_amount > amount_orig {
                break;
            }
            retries += 1;
            if retries > max_retries {
                return Err(ErrorInfo::error_info(format!("Failed to update eth_balance {status_info}")));
            }
        };
        Ok(())
    }

    //
    // pub async fn get_balance_multi(&self, addresses: Vec<structs::Address>) -> RgResult<HashMap<structs::Address, CurrencyAmount>> {
    //     let mut addrs = vec![];
    //     let mut mapping = HashMap::new();
    //     for addr in addresses {
    //         let s = addr.render_string()?;
    //         let aa = s.parse().error_info("address parse failure")?;
    //         mapping.insert(aa.clone(), addr);
    //         addrs.push(aa)
    //     }
    //     let metadata = self.client
    //         .get_ether_balance_multi(&*addrs, None).await.error_info("balance fetch failure")?;
    //     let mut res = HashMap::new();
    //     for m in metadata {
    //         let addr = mapping.get(&m.account).expect("address mapping");
    //         let amt = CurrencyAmount::from_eth_bigint_string(m.balance);
    //         res.insert(addr.clone(), amt);
    //     }
    //     Ok(res)
    // }

    pub fn translate_response_string_bigint(value: &String) -> RgResult<BigInt> {
        BigInt::from_str(value).error_info("value parse failure")
    }

    pub fn translate_value(value: &String) -> RgResult<i64> {
        BigInt::from_str(value).error_info("value parse failure")
            .map(|v| v / Self::bigint_offset())
            .and_then(|v| v.to_i64().ok_or(error_info("BigInt translation to i64 failure")))
    }
    pub fn translate_value_to_float(value: &String) -> RgResult<f64> {
        let bi = BigInt::from_str(value).error_info("bigint value parse failure")?;
        let f64 = bi.to_f64().ok_or(error_info("BigInt translation to f64 failure"))?;
        Ok(f64 / 10_f64.powi(18))
    }

    pub fn parse_address(value: &String) -> RgResult<structs::Address> {
        let addr: Address = value.parse().error_info("address parse failure")?;
        Ok(structs::Address::from_eth_direct(value))
    }

    // Workaround for dealing with u64's etc, drop from e18 precision to e8 precision
    pub fn bigint_offset() -> BigInt {
        BigInt::from(10_u64.pow(10))
    }

    pub fn translate_float_value(value: &String) -> RgResult<i64> {
        let f64_val = value.parse::<f64>().error_info(format!("failed to parse string value {} as f64", value))?;
        let f64_offset = f64_val * 10_f64.powi(18);
        let bi = BigInt::from_f64(f64_offset).ok_msg("f64 to BigInt failure")?;
        let offset_bi = bi / BigInt::from(10_u64.pow(10));
        offset_bi.to_i64().ok_or(error_info("BigInt translation to i64 failure"))
    }

    pub fn translate_float_value_bigint(value: &String) -> RgResult<BigInt> {
        let f64_val = value.parse::<f64>().error_info(format!("failed to parse string value {} as f64", value))?;
        let f64_offset = f64_val * 10_f64.powi(18);
        let bi = BigInt::from_f64(f64_offset).ok_msg("f64 to BigInt failure")?;
        Ok(bi)
    }

    pub fn translate_big_int_u256(value: BigInt) -> U256 {
        let u256 = U256::from_big_endian(&*value.to_bytes_be().1);
        u256
    }

    pub fn translate_ruint_u256_big_int(value: U256) -> BigInt {
        let mut vec = vec![];
        value.to_big_endian(&mut *vec);
        let bi = BigInt::from_bytes_be(Sign::Plus, &*vec);
        bi
    }

    pub fn translate_float_value_u256(value: &String) -> RgResult<U256> {
        let bi = Self::translate_float_value_bigint(&value)?;
        let u256 = Self::translate_big_int_u256(bi);
        Ok(u256)
    }


    pub fn translate_value_bigint(value: i64) -> RgResult<BigInt> {
        BigInt::from_i64(value).ok_or(error_info("BigInt int64 value parse failure"))
            .map(|v| v * BigInt::from(10_u64.pow(10)))
    }


    pub async fn get_all_tx_with_retries(
        &self,
        address: &String,
        start_block: Option<u64>,
        max_retries: Option<usize>,
        retry_interval_seconds: Option<usize>
    ) -> RgResult<Vec<ExternalTimedTransaction>> {
        let mut max_retries = max_retries.unwrap_or(3);
        let retry_interval_seconds = retry_interval_seconds.unwrap_or(10);
        loop {
            match self.get_all_tx(address, start_block).await {
                Ok(o) => { return Ok(o) }
                Err(e) => {
                    if e.lib_message.contains("Rate limit exceeded") {
                        info!("Rate limit exceeded, retrying in 10 seconds");
                    } else {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_secs(retry_interval_seconds as u64)).await;
                    max_retries -= 1;
                    if max_retries == 0 {
                        info!("Max retries exceeded for get_all_tx_with_retries");
                        return Err(e);
                    }
                }
            }
        }
    }


    pub async fn get_all_deployed_contracts(
        &self,
        address: &structs::Address,
        to_address: &structs::Address,
        start_block: Option<u64>
    ) -> RgResult<Vec<structs::Address>> {
        let address = address.render_string()?;
        let addr = address.parse().error_info("address parse failure")?;
        let tx_params = if let Some(s) = start_block {
            Some(TxListParams::new(s, 1e16 as u64, 0, 0, Sort::Asc))
        } else {
            None
        };
        let txs = self.client.get_transactions(&addr, tx_params)
            .await
            .error_info("EthHistoricalClient get_all_deployed_contracts transaction fetch failure")?;
        let mut res = vec![];
        let to_address = to_address.render_string()?;
        for t in txs {
            // if let Some(to) = t.to.as_ref() {
            //     if to_address == to.to_string() {
                    if let Some(c) = t.contract_address {
                        let addr = c.to_string();
                        let addr = structs::Address::from_eth_direct(&addr);
                        res.push(addr);
                    }
                // }
            // }
        }
        Ok(res)
    }

    pub async fn get_all_erc20_token_tx(
        &self,
        address: &structs::Address,
        start_block: Option<u64>
    ) -> RgResult<Vec<ExternalTimedTransaction>> {

        let query_address_string = address.render_string()?;
        let addr = query_address_string.clone().parse().error_info("address parse failure")?;

        let tx_params = if let Some(s) = start_block {
            Some(TxListParams::new(s, 1e16 as u64, 0, 0, Sort::Asc))
        } else {
            None
        };

        let token_query = TokenQueryOption::ByAddress(addr);
        let txs = self.client.get_erc20_token_transfer_events(token_query, tx_params).await
            .error_info("EthHistoricalClient get_all_tx transaction fetch failure")?;

        let mut res: Vec<ExternalTimedTransaction> = vec![];
        for t in txs.iter() {
            let tx_id = hex::encode(t.hash.0);
            let to_opt = t.to.map(|h| h.to_string());
            let timestamp = t.time_stamp.parse::<u64>().ok().map(|t| t * 1000);
            let block_num = t.block_number.as_number().map(|b| b.as_limbs()[0].clone());
            let value_str = t.value.to_string();
            let decimals = t.token_decimal.clone();
            let id = t.contract_address.to_string();
            let contract_addr = structs::Address::from_eth_external_exact(&id);
            let currency_id = CurrencyId::from_erc20(&contract_addr);
            let currency_amount = CurrencyAmount::from_eth_network_bigint_string_currency_id_decimals(
                value_str.clone(), currency_id.clone(), Some(decimals.clone()));
            let amount_i64 = currency_amount.to_1e8();
            let from = t.from.to_string();

            let other_address = if from == query_address_string {
                to_opt.clone().unwrap_or("".to_string())
            } else {
                from.clone()
            };

            let incoming = from != query_address_string;

            let gas = BigInt::from_str(&*t.gas_used.to_string()).error_info("BigInt parse failure on gas used")?;
            let gas_price = BigInt::from_str(&*t.gas_price.map(|g| g.to_string()).unwrap_or("0".to_string()))
                .error_info("BigInt parse failure on gas price")?;
            let fee = CurrencyAmount::from_eth_bigint(gas * gas_price);
            if fee.is_zero() {
                error!("Fee is zero for tx: {}", tx_id);
            }

            let to_dest = if let Some(t) = to_opt.clone() {
                let a = t.parse_ethereum_address()?;
                vec![(a, currency_amount.clone())]
            } else {
                vec![]
            };
            let ett = ExternalTimedTransaction {
                tx_id,
                timestamp,
                other_address: other_address.clone(),
                other_output_addresses: vec![],
                amount: amount_i64 as u64,
                bigint_amount: Some(value_str.clone()),
                incoming,
                currency: SupportedCurrency::Ethereum,
                block_number: block_num,
                price_usd: None,
                fee: Some(fee),
                self_address: Some(query_address_string.clone()),
                currency_id: Some(currency_id),
                currency_amount: Some(currency_amount.clone()),
                from: from.parse_ethereum_address()?,
                to: to_dest,
                other: Some(other_address.parse_ethereum_address()?),
                queried_address: Some(address.clone()),
            };
            res.push(ett);
        }
        /*


            //
            // let string = t.gas_used.to_string();
            // let gas_used = BigInt::from_str(&*string).error_info("BigInt parse failure on gas used")?;
            // let fee = t.gas_price.map(|p| p.to_string())
            //     .map(|gas_price| BigInt::from_str(&*gas_price).error_info("BigInt parse failure on gas price"))
            //     .transpose()?
            //     .map(|gp| gp * gas_used)
            //     .map(|ca| CurrencyAmount::from_eth_bigint(ca));
            //
            // if let (Some(tx_id), Some(from), Some(to)) = (tx_id, from_opt, to_opt) {
            //     let incoming = &to == address;
            //     let mut self_address = None;
            //     let other_address = if incoming {
            //         self_address = Some(to.clone());
            //         from
            //     } else {
            //         self_address = Some(from.clone());
            //         to
            //     };
            //     res.push(ExternalTimedTransaction {
            //         tx_id,
            //         timestamp,
            //         other_address,
            //         other_output_addresses: vec![],
            //         amount: amount as u64,
            //         bigint_amount: Some(value_str),
            //         incoming,
            //         currency: SupportedCurrency::Ethereum,
            //         block_number: block_num,
            //         price_usd: None,
            //         fee,
            //         self_address,
            //         currency_id: None,
            //     });
            // }
        }
         */
        Ok(res)
    }

    pub async fn get_all_raw_tx(
        &self,
        address: &structs::Address,
        start_block: Option<u64>
    ) -> RgResult<Vec<NormalTransaction>> {
        let address = address.render_string()?;
        let addr = address.parse().error_info("address parse failure")?;

        let tx_params = if let Some(s) = start_block {
            Some(TxListParams::new(s, 1e16 as u64, 0, 0, Sort::Asc))
        } else {
            None
        };
        let txs = self.client.get_transactions(&addr, tx_params).await
            .error_info("EthHistoricalClient get_all_tx transaction fetch failure")?;
        Ok(txs)
    }

    pub async fn get_all_tx(
        &self,
        address: &String,
        start_block: Option<u64>
    ) -> RgResult<Vec<ExternalTimedTransaction>> {
        let addr = address.parse().error_info("address parse failure")?;

        let tx_params = if let Some(s) = start_block {
            Some(TxListParams::new(s, 1e16 as u64, 0, 0, Sort::Asc))
        } else {
            None
        };
        let txs = self.client.get_transactions(&addr, tx_params).await
            .error_info("EthHistoricalClient get_all_tx transaction fetch failure")?;
        let mut res = vec![];
        for t in txs {
            let tx_id = match t.hash {
                GenesisOption::Some(h) => {Some(hex::encode(h.0))}
                _ => {None}
            };
            let from_opt = match t.from {
                GenesisOption::Some(h) => {Some(h.to_string())}
                _ => {None}
            };
            let to_opt = t.to.map(|h| h.to_string());
            let timestamp = t.time_stamp.parse::<u64>().ok().map(|t| t * 1000);
            let block_num = t.block_number.as_number().map(|b| b.as_limbs()[0].clone());

            let value_str = t.value.to_string();
            let amount = Self::translate_value(&value_str)?;

            let string = t.gas_used.to_string();
            let gas_used = BigInt::from_str(&*string).error_info("BigInt parse failure on gas used")?;
            let fee = t.gas_price.map(|p| p.to_string())
                .map(|gas_price| BigInt::from_str(&*gas_price).error_info("BigInt parse failure on gas price"))
                .transpose()?
                .map(|gp| gp * gas_used)
                .map(|ca| CurrencyAmount::from_eth_bigint(ca));

            if let (Some(tx_id), Some(from), Some(to)) = (tx_id, from_opt, to_opt) {
                let incoming = &to == address;
                let mut self_address = None;
                let other_address = if incoming {
                    self_address = Some(to.clone());
                    from.clone()
                } else {
                    self_address = Some(from.clone());
                    to.clone()
                };
                let amount_t = CurrencyAmount::from_eth_bigint_string(value_str.clone());
                res.push(ExternalTimedTransaction {
                    tx_id,
                    timestamp,
                    other_address: other_address.clone(),
                    other_output_addresses: vec![],
                    amount: amount as u64,
                    bigint_amount: Some(value_str.clone()),
                    incoming,
                    currency: SupportedCurrency::Ethereum,
                    block_number: block_num,
                    price_usd: None,
                    fee,
                    self_address,
                    currency_id: Some(SupportedCurrency::Ethereum.into()),
                    currency_amount: Some(amount_t.clone()),
                    from: structs::Address::from_eth_external_exact(&from),
                    to: vec![(structs::Address::from_eth_external_exact(&to), amount_t)],
                    other: Some(structs::Address::from_eth_external_exact(&other_address)),
                    queried_address: Some(structs::Address::from_eth_external_exact(address)),
                });
            }
        }
        Ok(res)
    }

}


// #[ignore]
#[tokio::test]
async fn show_balances() {
    let (dev_secret, dev_kp) = example::dev_ci_kp().expect("works");
    let addr = dev_kp.public_key().to_ethereum_address_typed().expect("works");
    let environment = NetworkEnvironment::Dev;
    let h = EthHistoricalClient::new(&environment).expect("works").expect("works");
    let b = h.get_balance_typed(&addr).await.expect("works");
    println!("balance: {}", b.json_or());

    let addr_str = addr.render_string().expect("works");
    let tx = h.get_all_tx(&addr_str, None).await.expect("works");

    for t in tx {
        println!("tx: {}", t.json_or());
    }
    // let fee = h.recommended_fee_typed().await.expect("works");
    // println!("fee: {}", fee.json_or());
}
