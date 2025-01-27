use std::ops::Index;
use std::sync::Arc;
use ethers::abi::Address;
use ethers::middleware::SignerMiddleware;
use ethers::prelude::{Http, LocalWallet, Provider, Signer, TransactionRequest};
use redgold_safe_bindings::safe;
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::CurrencyAmount;
use crate::eth::eth_wallet::EthWalletWrapper;
use ethers::core::types::U256;
use ethers::types::{Bytes, RecoveryMessage, H256};
use redgold_keys::util::sign;
use redgold_schema::errors::into_error::ToErrorInfo;

impl EthWalletWrapper {

    pub async fn safe(&self, addr: &structs::Address) -> RgResult<safe::safe<SignerMiddleware<Provider<Http>, LocalWallet>>> {
        let addr = addr.render_string()?;
        let safe_address = addr.parse::<Address>().error_info("parse")?;
        let res = safe::safe::new(safe_address, Arc::new(self.client.clone()));
        Ok(res)
    }

    pub async fn get_transaction_data(
        &self,
        safe_contract_address: &structs::Address,
        to: &structs::Address,
        amount: &CurrencyAmount
    ) -> RgResult<(Bytes, [u8; 32])> { // Return transaction data and signature
        // Get the Safe contract instance
        let this_safe = self.safe(safe_contract_address).await?;

        // Convert destination address
        let to_address = to.render_string()?.parse::<Address>().error_info("parse to address")?;

        // Convert amount to Wei (assuming amount is in ETH)
        let string_amount = amount.string_amount.clone().ok_msg("Missing amount")?;
        let amount_wei = U256::from_dec_str(&*string_amount).error_info("parse amount")?;

        // Get the nonce for the Safe transaction
        let nonce = this_safe.nonce().call().await.error_info("fetch nonce")?;


        // Create Safe transaction data
        let tx_data = this_safe
            .encode_transaction_data(
                to_address,           // to
                amount_wei,           // value
                Bytes::default(),     // data (empty for simple ETH transfer)
                0u8,                  // operation (0 for call)
                U256::zero(),         // safeTxGas
                U256::zero(),         // baseGas
                U256::zero(),         // gasPrice
                Address::zero(),      // gasToken
                Address::zero(),      // refundReceiver
                nonce                 // _nonce
            )
            .call()
            .await
            .error_info("encode transaction")?;


        // Get the transaction hash
        let tx_hash = this_safe
            .get_transaction_hash(
                to_address,           // to
                amount_wei,           // value
                Bytes::default(),     // data (empty for simple ETH transfer)
                0u8,                  // operation (0 for call)
                U256::zero(),         // safeTxGas
                U256::zero(),         // baseGas
                U256::zero(),         // gasPrice
                Address::zero(),      // gasToken
                Address::zero(),      // refundReceiver
                nonce                 // nonce
            )
            .call()
            .await
            .error_info("get transaction hash")?;


        Ok((tx_data, tx_hash))
    }
    pub async fn sign_safe_tx(
        &self,
        safe_addr: &structs::Address,
        to: &structs::Address,
        amount: &CurrencyAmount,
    ) -> RgResult<Bytes> {
        let safe = self.safe(safe_addr).await?;

        // Convert addresses and amount
        let to_address = to.render_string()?.parse::<Address>().error_info("parse to address")?;
        let amount_wei = U256::from_dec_str(&amount.string_amount.clone().ok_msg("Missing amount")?)
            .error_info("parse amount")?;

        // Get the nonce
        let nonce = safe.nonce().call().await.error_info("fetch nonce")?;

        // Get the transaction hash
        let tx_hash = safe
            .get_transaction_hash(
                to_address,           // to
                amount_wei,           // value
                Bytes::default(),     // data (empty for simple ETH transfer)
                0u8,                  // operation (0 for call)
                U256::zero(),         // safeTxGas
                U256::zero(),         // baseGas
                U256::zero(),         // gasPrice
                Address::zero(),      // gasToken
                Address::zero(),      // refundReceiver
                nonce                 // nonce
            )
            .call()
            .await
            .error_info("get transaction hash")?;

        // Sign the transaction hash
        let signature = self.client.signer()
            .sign_message(tx_hash)
            .await
            .error_info("sign transaction")?;

        Ok(signature.to_vec().into())

    }
    pub async fn execute_safe_transaction(
        &self,
        safe_addr: &structs::Address,
        to: &structs::Address,
        amount: &CurrencyAmount,
        signatures: Vec<Bytes>
    ) -> RgResult<()> {
        let safe = self.safe(safe_addr).await?;

        let (tx_data, tx_hash) = self.get_transaction_data(safe_addr, to, amount).await?;

        let combined_signatures = Self::combine_signatures(tx_hash.into(), signatures)?;

        // Convert destination address
        let to_address = to.render_string()?.parse::<Address>().error_info("parse to address")?;

        // Convert amount to Wei
        let amount_wei = U256::from_dec_str(&amount.string_amount.clone().ok_msg("Missing amount")?)
            .error_info("parse amount")?;

        // Execute transaction with collected signatures
        safe.exec_transaction(
            to_address,           // destination address
            amount_wei,           // amount in wei
            Bytes::default(),     // data (empty for simple ETH transfer)
            0u8,                  // operation (0 for Call)
            U256::zero(),         // safeTxGas
            U256::zero(),         // baseGas
            U256::zero(),         // gasPrice
            Address::zero(),      // gasToken
            Address::zero(),      // refundReceiver
            combined_signatures   // combined signatures
        )
            .send()
            .await
            .error_info("Failed to execute transaction")?;

        Ok(())
    }
    pub fn combine_signatures(
        tx_hash: H256,
        signatures: Vec<Bytes>
    ) -> RgResult<Bytes> {
        // First recover signer addresses from signatures to sort them
        let mut sigs_with_addresses: Vec<(Address, Bytes)> = signatures
            .into_iter()
            .map(|sig| {
                // Each signature is 65 bytes: r (32) + s (32) + v (1)
                if sig.len() != 65 {
                    return "Invalid signature length".to_error();
                }

                let signature = ethers::core::types::Signature::try_from(sig.as_ref())
                    .error_info("Failed to create signature")?;

                // Recover the address using ecrecover
                let recovered = signature
                    .recover(tx_hash)
                    .error_info("Failed to recover address")?;

                Ok((recovered, sig))
            })
            .collect::<RgResult<Vec<_>>>()?;

        // Sort by signer address
        sigs_with_addresses.sort_by(|a, b| a.0.cmp(&b.0));

        // Concatenate sorted signatures
        let mut combined = Vec::new();
        for (_, sig) in sigs_with_addresses {
            combined.extend_from_slice(&sig);
        }

        Ok(combined.into())
    }
    // Add helper method to decode transaction data
    // pub fn decode_safe_tx_data(tx_data: &[u8]) -> RgResult<(Address, U256, Bytes, u8)> {
    //     if tx_data.len() < 100 {
    //         "Transaction data too short".to_error()?
    //     }
    //
    //     let to = Address::from_slice(&tx_data[16..36]);
    //     let value = U256::from_big_endian(&tx_data[36..68]);
    //     let data_length = U256::from_big_endian(&tx_data[68..100]).as_usize();
    //     let data = if data_length > 0 && tx_data.len() >= 100 + data_length {
    //         Bytes::from(&tx_data[100..100 + data_length])
    //     } else {
    //         Bytes::default()
    //     };
    //     let operation = 0u8;  // Default to Call operation
    //
    //     Ok((to, value, data, operation))
    // }
}