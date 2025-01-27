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
use ethers::providers::Middleware;
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
    //
    // pub async fn gas_estimate(
    //     &self,
    //     safe_addr: &structs::Address,
    //     to: &structs::Address,
    //     amount: &CurrencyAmount,
    // ) -> RgResult<U256> {
    //     let safe = self.safe(safe_addr).await?;
    //     let to_address = to.render_string()?.parse::<Address>().error_info("parse to address")?;
    //     let amount_wei = U256::from_dec_str(&amount.string_amount.clone().ok_msg("Missing amount")?)
    //         .error_info("parse amount")?;
    //
    //     let data = safe.encode_transaction_data(
    //         to_address,
    //         amount_wei,
    //         Bytes::default(),
    //         0u8,
    //         U256::zero(),
    //         U256::zero(),
    //         U256::zero(),
    //         Address::zero(),
    //         Address::zero(),
    //         U256::zero()
    //     ).call().await.error_info("encode transaction data")?;
    //
    //     let estimate = self.provider
    //         .estimate_gas(
    //             &TransactionRequest {
    //                 from:   Some(self.client.address().into()),
    //                 to:     Some(safe_address.into()),
    //                 data:   Some(data),
    //                 value:  Some(U256::zero()),
    //                 ..Default::default()
    //             }
    //         )
    //         .await
    //         .error_info("estimate gas")?;
    //     Ok(estimate)
    // }

    pub async fn sign_safe_tx(
        &self,
        safe_addr: &structs::Address,
        to: &structs::Address,
        amount: &CurrencyAmount,
    ) -> RgResult<([u8; 32], Bytes)> {
        let safe = self.safe(safe_addr).await?;

        let to_address = to.render_string()?.parse::<Address>().error_info("parse to address")?;
        let amount_wei = U256::from_dec_str(&amount.string_amount.clone().ok_msg("Missing amount")?)
            .error_info("parse amount")?;

        let nonce = safe.nonce().call().await.error_info("fetch nonce")?;


        // // Get the domain separator
        // let domain_separator = safe.domain_separator().call().await.error_info("get domain separator")?;
        // println!("Domain separator: 0x{}", hex::encode(domain_separator));

        // Get the safe transaction hash
        let tx_hash = safe
            .get_transaction_hash(
                to_address,
                amount_wei,
                Bytes::default(),
                0u8,
                U256::zero(),
                U256::zero(),
                U256::zero(),
                // U256::from(150000),    // safe_tx_gas - estimate for ETH transfer
                // U256::from(121000),    // base_gas - standard ETH transfer cost
                // U256::from(1),        // gas_price - minimum non-zero value
                Address::zero(),
                Address::zero(),
                nonce
            )
            .call()
            .await
            .error_info("get transaction hash")?;

        // println!("Safe tx hash: 0x{}", hex::encode(tx_hash));

        // Sign the hash
        let signature = self.client.signer()
            .sign_hash(H256::from(tx_hash))
            .error_info("sign transaction")?;

        let sig_bytes = signature.to_vec();
        // println!("Signature: 0x{}", hex::encode(&sig_bytes));

        Ok((tx_hash, sig_bytes.into()))
    }

    pub fn combine_signatures(
        message_hash: [u8; 32],
        signatures: Vec<Bytes>
    ) -> RgResult<Bytes> {

        let rec_message = RecoveryMessage::Hash(H256::from(message_hash));
        // let rec_message = RecoveryMessage::Data(message_hash.to_vec());
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
                    .recover(rec_message.clone())
                    .error_info("Failed to recover address")?;

                Ok((recovered, sig))
            })
            .collect::<RgResult<Vec<_>>>()?;

        // Sort by signer address
        sigs_with_addresses.sort_by(|a, b| a.0.cmp(&b.0));
        println!("Sorted signer addresses: {:?}", sigs_with_addresses.iter().map(|(addr, _)| addr).collect::<Vec<_>>());

        // Concatenate sorted signatures
        let mut combined = Vec::new();
        for (_, sig) in sigs_with_addresses {
            combined.extend_from_slice(&sig);
        }

        Ok(combined.into())
    }
    pub async fn execute_safe_transaction(
        &self,
        safe_addr: &structs::Address,
        to: &structs::Address,
        amount: &CurrencyAmount,
        signatures: Vec<Bytes>,
        message_hash: [u8; 32],
    ) -> RgResult<String> {
        let safe = self.safe(safe_addr).await?;


        // Log threshold and owners
        let threshold = safe.get_threshold().call().await.error_info("threshold")?;
        let owners = safe.get_owners().call().await.error_info("owners")?;
        println!("Safe threshold: {}", threshold);
        println!("Safe owners: {:?}", owners);

        // Log number of signatures
        println!("Number of signatures provided: {}", signatures.len());

        let combined_signatures = Self::combine_signatures(message_hash, signatures)?;

        // Convert destination address
        let to_address = to.render_string()?.parse::<Address>().error_info("parse to address")?;

        // Convert amount to Wei
        let amount_wei = U256::from_dec_str(&amount.string_amount.clone().ok_msg("Missing amount")?)
            .error_info("parse amount")?;
        // Execute transaction with collected signatures
        let call = safe.exec_transaction(
            to_address,           // destination address
            amount_wei,           // amount in wei
            Bytes::default(),     // data (empty for simple ETH transfer)
            0u8,                  // operation (0 for Call)
            U256::zero(),
            U256::zero(),
            U256::zero(),
            // U256::from(150000),    // safe_tx_gas - estimate for ETH transfer
            // U256::from(121000),    // base_gas - standard ETH transfer cost
            // U256::from(1),        // gas_price - minimum non-zero value
            Address::zero(),      // gasToken
            Address::zero(),      // refundReceiver
            combined_signatures   // combined signatures
        )
            .gas(150_000u64);
        let result = call
            .send()
            .await
            .error_info("Failed to execute transaction")?;
        let h256 = result.tx_hash();
        Ok(hex::encode(h256.0))

    }

}