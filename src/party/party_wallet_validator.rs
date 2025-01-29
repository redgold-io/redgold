use crate::core::relay::Relay;
use crate::party::party_stream::PartyEventBuilder;
use async_trait::async_trait;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::database::BatchDatabase;
use itertools::Itertools;
use redgold_keys::btc::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::structs::{NetworkEnvironment, SupportedCurrency};
use redgold_schema::{error_info, RgResult};

#[async_trait]
pub trait PartyWalletMethods {
    fn validate_btc_fulfillment<D: BatchDatabase>(
        &self,
        validation_payload: String,
        signing_data: Vec<u8>,
        w: &mut SingleKeyBitcoinWallet<D>
    ) -> RgResult<()>;
    async fn validate_eth_fulfillment(&self, typed_tx_payload: String, signing_data: Vec<u8>, r: &Relay) -> RgResult<()>;
}

#[async_trait]
impl PartyWalletMethods for PartyEvents {
    fn validate_btc_fulfillment<D: BatchDatabase>(
        &self,
        validation_payload: String,
        signing_data: Vec<u8>,
        w: &mut SingleKeyBitcoinWallet<D>
    ) -> RgResult<()> {
        //
        // let btc = self.fulfillment_orders(SupportedCurrency::Bitcoin)
        //     .iter()
        //     .map(|o| (o.destination.render_string().expect("works"), o.fulfilled_currency_amount().amount_i64_or() as u64))
        //     .collect_vec();
        //
        // let psbt: PartiallySignedTransaction = validation_payload.clone().json_from()?;
        // w.psbt = Some(psbt.clone());
        // let has_expected_hash = w.signable_hashes()?.iter().filter(|(h, _)| h == &signing_data).next().is_some();
        // if !has_expected_hash {
        //     return Err(error_info("Invalid BTC fulfillment signing data"))
        //         .with_detail("signing_data", hex::encode(signing_data))
        //         .with_detail("payload", validation_payload);
        // }
        //
        // let party_self = self.party_public_key.to_all_addresses()?.iter().flat_map(|a| a.render_string().ok()).collect_vec();
        // let outs = w.convert_psbt_outputs();
        // for (out_addr, out_amt) in outs {
        //     if party_self
        //         .iter()
        //         .find(|&a| a == &out_addr).is_some() {
        //         continue;
        //     }
        //     if btc.iter().find(|(addr, amt) | addr == &out_addr && {
        //         let this_amt = *amt as i64;
        //         let out_amt_i64 = out_amt as i64;
        //         let within_reasonable_range = i64::abs(this_amt - out_amt_i64) < 10_000;
        //         within_reasonable_range
        //     }).is_none() {
        //         let has_matching_address = btc.iter().find(|(addr, amt) | addr == &out_addr).is_some();
        //         let err = Err(error_info("Invalid BTC fulfillment output"))
        //             .with_detail("output_address", out_addr)
        //             .with_detail("has_matching_address", has_matching_address.to_string())
        //             .with_detail("btc_orders_len", btc.len().to_string())
        //             .with_detail("output_amount", out_amt.to_string())
        //             .with_detail("btc_orders", btc.json_or());
        //         if self.network.clone() != NetworkEnvironment::Debug {
        //             // err.log_error().ok();
        //             return err
        //         }
        //     }
        // }
        Ok(())
    }


    async fn validate_eth_fulfillment(&self, typed_tx_payload: String, signing_data: Vec<u8>, r: &Relay) -> RgResult<()> {
        // let fulfills = self.fulfillment_orders(SupportedCurrency::Ethereum)
        //     .into_iter()
        //     .map(|o| {
        //         (o.destination.clone(), o.fulfilled_currency_amount())
        //     }).collect_vec();
        // let w = r.eth_wallet()?;
        // EthWalletWrapper::validate_eth_fulfillment(fulfills, &typed_tx_payload, &signing_data, &self.network, &w).await?;

        Ok(())
    }

}