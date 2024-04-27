use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bdk::database::MemoryDatabase;
use itertools::Itertools;
use log::{error, info};
use serde::{Deserialize, Serialize};
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::eth::example::EthWalletWrapper;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::{error_info, RgResult, SafeOption, structs};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::{Address, BytesData, CurrencyAmount, ErrorInfo, ExternalTransactionId, Hash, MultipartyIdentifier, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction, UtxoEntry};
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::multiparty_gg20::initiate_mp::initiate_mp_keysign;
use crate::party::data_enrichment::PartyInternalData;
use crate::party::party_stream::PartyEvents;
use crate::party::party_watcher::PartyWatcher;
use crate::party::price_volume::PriceVolume;
use crate::util::current_time_millis_i64;

impl PartyWatcher {
    pub async fn handle_order_fulfillment(&self, data: &mut HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {
        for (key,v ) in data.iter_mut() {
            if !v.party_info.self_initiated.unwrap_or(false) {
                continue;
            }
            if v.party_info.successor_key.is_some() {
                continue;
            }
            if let Some(ps) = v.party_events.as_mut() {
                let key_address = key.address()?;
                let btc_starting_balance = v.network_data.get(&SupportedCurrency::Bitcoin)
                    .map(|d| d.balance.amount).unwrap_or(0);

                let cutoff_time = current_time_millis_i64() - self.relay.node_config.config_data.party_config_data.order_cutoff_delay_time; //
                let orders = ps.orders();
                let cutoff_orders = ps.orders().iter().filter(|o| o.event_time < cutoff_time).cloned().collect_vec();
                let identifier = v.party_info.initiate.safe_get()?.identifier.safe_get().cloned()?;
                let environment = self.relay.node_config.network.clone();
                let btc_address = key.to_bitcoin_address(&environment)?;

                let balance = self.relay.ds.transaction_store.get_balance(&key_address).await?;
                let rdg_starting_balance: i64 = balance.safe_get_msg("Missing balance")?.clone();


                let num_events = ps.events.len();
                let num_unconfirmed = ps.unconfirmed_events.len();
                let num_unfulfilled_deposits = ps.unfulfilled_deposits.len();
                let num_unfulfilled_withdrawals = ps.unfulfilled_withdrawals.len();
                let utxos = self.relay.ds.transaction_store.query_utxo_address(&key_address).await?;

                info!("watcher balances: RDG:{}, BTC:{} \
         BTC_address: {} environment: {} orders {} cutoff_orders {} num_events: {} num_unconfirmed {} num_un_deposit {} \
         num_un_withdrawls {} num_utxos: {}  \
         central_prices: {} orders_json {}",
            rdg_starting_balance, btc_starting_balance, btc_address, environment.to_std_string(),
            orders.len(),
            cutoff_orders.len(),
            num_events, num_unconfirmed, num_unfulfilled_deposits, num_unfulfilled_withdrawals,
            utxos.len(),
            ps.central_prices.json_or(),
            orders.json_or(),
        );

                self.fulfill_btc_orders(key, &identifier, ps, cutoff_time).await?;

                self.fulfill_eth(ps, &identifier).await?;

                self.fulfill_rdg_orders(&identifier, &utxos, ps, cutoff_time).await?;


            }
        }
        Ok(())
    }

    async fn fulfill_btc_orders(&self, key: &PublicKey, identifier: &MultipartyIdentifier, ps: &PartyEvents, cutoff_time: i64) -> RgResult<()> {
        let btc_outputs = ps.orders().iter()
            .filter(|e| e.event_time < cutoff_time)
            .filter(|e| !e.is_ask_fulfillment_from_external_deposit)
            .filter(|e| e.destination.currency_or() == SupportedCurrency::Bitcoin)
            .map(|o| {
                let btc = o.destination.render_string().expect("works");
                let amount = o.fulfilled_amount;
                let outputs = (btc, amount);
                outputs
            }).collect_vec();

        if btc_outputs.len() > 0 {
            let txid = self.mp_send_btc(key, &identifier, btc_outputs.clone(), ps).await.log_error().ok();
            info!("Sending BTC fulfillment transaction id {}: {:?}", txid.json_or(), btc_outputs);
        }
        Ok(())
    }

    async fn fulfill_rdg_orders(&self, identifier: &MultipartyIdentifier, utxos: &Vec<UtxoEntry>, ps: &PartyEvents, cutoff_time: i64) -> Result<(), ErrorInfo> {
        let mut tb = TransactionBuilder::new(&self.relay.node_config);
        tb.with_utxos(&utxos)?;

        let rdg_fulfillment_txb = ps.orders().iter()
            .filter(|e| e.event_time < cutoff_time)
            .filter(|e| e.is_ask_fulfillment_from_external_deposit && e.tx_id_ref.is_some())
            .fold(&mut tb, |tb, o| {
                tb.with_output(&o.destination, &o.fulfilled_currency_amount())
                    .with_last_output_deposit_swap_fulfillment(o.tx_id_ref.clone().expect("Missing tx_id")).expect("works")
            });

        if rdg_fulfillment_txb.transaction.outputs.len() > 0 {
            let tx = rdg_fulfillment_txb.build()?;
            ps.validate_rdg_swap_fulfillment_transaction(&tx)?;
            info!("Sending RDG fulfillment transaction: {}", tx.json_or());
            self.mp_send_rdg_tx(&mut tx.clone(), identifier.clone()).await.log_error().ok();
        }
        Ok(())
    }


    pub async fn fulfill_eth(&self, pes: &PartyEvents, ident: &MultipartyIdentifier) -> RgResult<()> {
        let orders = pes.orders();
        let eth_orders = orders.iter()
            .filter(|o| o.destination.currency_or() == SupportedCurrency::Ethereum)
            .collect_vec();
        let eth = self.relay.eth_wallet()?;
        let mp_eth_addr = pes.party_public_key.to_ethereum_address()?;

        for order in eth_orders {
            if order.destination.currency_or() != SupportedCurrency::Ethereum {
                error!("Invalid currency for fulfillment: {}", order.json_or());
                continue;
            }
            let dest = order.destination.render_string()?;
            let mut tx = eth.create_transaction(
                &mp_eth_addr, &dest, order.fulfilled_currency_amount().amount_i64_or() as u64
            ).await?;
            let data = EthWalletWrapper::signing_data(&tx)?;
            let tx_ser = tx.json_or();
            let mut valid = structs::PartySigningValidation::default();
            valid.json_payload = Some(tx_ser);
            valid.currency = SupportedCurrency::Ethereum as i32;
            let res = initiate_mp_keysign(
                self.relay.clone(), ident.clone(), BytesData::from(data), ident.party_keys.clone(), None,
                Some(valid)
            ).await?;
            let sig = res.proof.signature.ok_msg("Missing keysign result signature")?;
            let raw = EthWalletWrapper::process_signature(sig, &mut tx)?;
            eth.broadcast_tx(raw).await?;
        }
        Ok(())
    }

    pub async fn mp_send_btc(&self, public_key: &PublicKey, identifier: &MultipartyIdentifier, outputs: Vec<(String, u64)>, ps: &PartyEvents) -> RgResult<String> {
        let mut w = self.relay.btc_wallet(public_key)?;
        w.create_transaction_output_batch(outputs)?;

        let pbst_payload = w.psbt.safe_get_msg("Missing PSBT")?.clone().json_or();
        let mut validation = structs::PartySigningValidation::default();
        validation.json_payload = Some(pbst_payload.clone());
        validation.currency = SupportedCurrency::Bitcoin as i32;

        let hashes = w.signable_hashes()?.clone();
        for (i, (hash, hash_type)) in hashes.iter().enumerate() {

            ps.validate_btc_fulfillment(pbst_payload.clone(), hash.clone(), &mut w)?;
            let result = initiate_mp_keysign(self.relay.clone(), identifier.clone(),
                                             BytesData::from(hash.clone()),
                                             identifier.party_keys.clone(), None,
                Some(validation.clone())
            ).await?;
            w.affix_input_signature(i, &result.proof, hash_type);
        }
        w.sign()?;
        w.broadcast_tx()?;
        Ok(w.txid()?)
    }
    pub async fn mp_send_rdg_tx(&self, tx: &mut Transaction, identifier: MultipartyIdentifier) -> RgResult<SubmitTransactionResponse> {
        let mut validation = structs::PartySigningValidation::default();
        validation.transaction = Some(tx.clone());
        validation.currency = SupportedCurrency::Redgold as i32;

        let hash = tx.signable_hash();
        let result = initiate_mp_keysign(self.relay.clone(), identifier.clone(),
                                         hash.bytes.safe_get()?.clone(), identifier.party_keys.clone(), None,
                                            Some(validation.clone())
        ).await?;
        tx.add_proof_per_input(&result.proof);
        self.relay.submit_transaction_sync(tx).await
    }

}

#[derive(Serialize, Deserialize, Clone)]
pub struct OrderFulfillment {
    pub order_amount: u64,
    pub fulfilled_amount: u64,
    pub updated_curve: Vec<PriceVolume>,
    pub is_ask_fulfillment_from_external_deposit: bool,
    pub event_time: i64,
    pub tx_id_ref: Option<ExternalTransactionId>,
    pub destination: Address
}

impl OrderFulfillment {
    pub fn fulfillment_price(&self) -> f64 {
        self.fulfilled_amount as f64 / self.order_amount as f64
    }

    pub fn fulfillment_fraction(&self) -> f64 {
        let total = self.fulfilled_amount + self.updated_curve.iter().map(|v| v.volume).sum::<u64>();
        let fraction = self.fulfilled_amount as f64 / total as f64;
        fraction
    }

    pub fn fulfilled_currency_amount(&self) -> CurrencyAmount {
        let c = self.destination.currency_or();
        if c == SupportedCurrency::Ethereum {
            CurrencyAmount::from_eth_i64(self.fulfilled_amount as i64)
        } else {
            CurrencyAmount::from_currency(self.fulfilled_amount as i64, c)
        }
    }



}