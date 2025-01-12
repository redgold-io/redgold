use std::str::FromStr;
use ethers::middleware::Middleware;
use ethers::prelude::{Address, Provider, TransactionRequest, U256};
use ethers::providers;
use ethers::providers::Http;
use ethers::types::transaction::eip2718::TypedTransaction;
use redgold_schema::{RgResult, structs, ErrorInfoContext, SafeOption, error_info};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey, SupportedCurrency};
use redgold_keys::address_external::ToEthereumAddress;
use crate::eth::historical_client::EthHistoricalClient;

pub struct EthWalletWrapperOffline {
    pub provider: Provider<Http>,
    pub network: NetworkEnvironment,
    pub chain_id: u64,
}

impl EthWalletWrapperOffline {
    pub fn new(network: &NetworkEnvironment) -> RgResult<EthWalletWrapperOffline> {
        let provider = if network.is_main() {
            &providers::MAINNET
        } else {
            &providers::SEPOLIA
        }.provider();
        let chain = EthHistoricalClient::chain_id(network).id();

        Ok(Self {
            provider,
            network: network.clone(),
            chain_id: chain,
        })
    }

    pub fn parse_address(a: &String) -> RgResult<Address> {
        let addr: Address = a.parse().error_info("address parse failure")?;
        Ok(addr)
    }

    pub async fn create_transaction_for_offline_signing(
        &self,
        from: &PublicKey,
        to: &structs::Address,
        value: CurrencyAmount,
        fee_gas_price: Option<CurrencyAmount>
    ) -> RgResult<TypedTransaction> {
        let from_address = from.to_ethereum_address_typed()?;
        self.create_transaction_typed_inner(&from_address, to, value, fee_gas_price).await
    }

    async fn create_transaction_typed_inner(
        &self,
        from: &structs::Address,
        to: &structs::Address,
        value: CurrencyAmount,
        fee_gas_price: Option<CurrencyAmount>
    ) -> RgResult<TypedTransaction> {
        if value.currency_or() != SupportedCurrency::Ethereum {
            return Err(error_info("Currency must be Ethereum"));
        }
        let fee_gas_price = fee_gas_price.unwrap_or(CurrencyAmount::gas_price_fixed_normal_by_env(&self.network));
        let big_value = value.bigint_amount().ok_msg("CurrencyAmount bigint amount missing")?;
        let bigint_str = big_value.to_string();
        let u256 = U256::from_dec_str(&*bigint_str).error_info("U256 parse failure")?;
        let to_address: Address = to.render_string()?.parse().error_info("to address parse failure")?;
        let from_address: Address = from.render_string()?.parse().error_info("from address parse failure")?;
        let tr = TransactionRequest::new().to(to_address).value(u256);
        let mut tx: TypedTransaction = tr.into();
        let gas_price_str = fee_gas_price.string_amount.ok_msg("fee gas price missing")?;
        let gas_price_u256 = U256::from_dec_str(&*gas_price_str).error_info("U256 parse failure")?;
        tx.set_gas_price(gas_price_u256);

        tx.set_chain_id(self.chain_id);
        tx.set_from(from_address);

        let nonce = self.provider.get_transaction_count(from_address, None).await
            .error_info("nonce get failure")?;
        tx.set_nonce(nonce);

        self.provider.fill_transaction(&mut tx, None).await
            .error_info("tx fill failure")
            .with_detail("tx", tx.json_or())?;

        Ok(tx)
    }

    pub fn get_signing_data(tx: &TypedTransaction) -> RgResult<Vec<u8>> {
        let sig_hash = tx.sighash().0.to_vec();
        Ok(sig_hash)
    }

    // Other methods remain the same...
}