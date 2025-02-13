use crate::party::party_stream::PartyEventBuilder;
use crate::party::party_watcher::PartyWatcher;
use crate::util::current_time_millis_i64;
use bdk::bitcoin::EcdsaSighashType;
use bdk::database::MemoryDatabase;
use itertools::Itertools;
use metrics::{counter, gauge};
use redgold_common::external_resources::{EncodedTransactionPayload, ExternalNetworkResources, PeerBroadcast};
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::btc::btc_wallet::SingleKeyBitcoinWallet;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::party_events::{OrderFulfillment, PartyEvents};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::party::price_volume::PriceVolume;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, AddressDescriptor, BytesData, CurrencyAmount, ErrorInfo, ExternalTransactionId, Hash, MultipartyIdentifier, MultisigRequest, MultisigResponse, NetworkEnvironment, PartySigningValidation, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction, UtxoEntry, UtxoId};
use redgold_schema::message::Request;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::{error_info, structs, RgResult, SafeOption};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{error, info};
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::word_pass_support::NodeConfigKeyPair;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::core::relay::Relay;
use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
use crate::util;

pub async fn handle_multisig_request<E: ExternalNetworkResources>(
    multisig_request: &MultisigRequest,
    relay: &Relay,
    ext: &E
) -> RgResult<MultisigResponse> {
    let pk = multisig_request.proposer_party_key.safe_get_msg("Missing proposer party key")?;
    let data = relay.external_network_shared_data.clone_read().await;
    let party = data.get(&pk).ok_msg("Missing party")?;
    let party_events = party.party_events.as_ref().ok_msg("Missing party events")?;
    let party_address = multisig_request.mp_address.safe_get_msg("Missing mp address")?;
    let party_instance = party.metadata.instance_of_address(party_address).ok_msg("Missing instances of address")?;
    let party_members = party.metadata.members_of(party_address);

    let dest = multisig_request.destination.safe_get_msg("Missing destination")?;
    let amount = multisig_request.amount.safe_get_msg("Missing amount")?;
    let orders = party_events.orders();
    let orders = orders.iter()
        .filter(|o| &o.destination == dest)
        .collect_vec();
    let mut valid = false;
    for o in orders {
        let amt = o.fulfilled_amount_typed.clone();
        let delta = amt.to_fractional() - amount.to_fractional();
        let abs = f64::abs(delta);
        let pct = abs / amt.to_fractional();
        if pct < 0.1f64 {
            valid = true;
        }
    }

    if !valid {
        "Invalid amount".to_error()?;
    }

    let cur = dest.currency_or();

    if cur == SupportedCurrency::Redgold {
        let mut tx = multisig_request.tx.safe_get_msg("Missing tx")?.clone();
        tx = tx.sign_multisig(&relay.node_config.keypair(), party_address)?;
        let mut mr = MultisigResponse::default();
        mr.tx = Some(tx.clone());
        mr.currency = cur as i32;
        return Ok(mr)
    }

    let mut response = ext.participate_multisig_send(
                multisig_request.clone(),
                &party_members,
                party_instance.threshold.safe_get_msg("Missing threshold")?.value
    ).await?;
    response.currency = cur as i32;
    Ok(response)
}

impl<T> PartyWatcher<T> where T: ExternalNetworkResources + Send {
    pub async fn handle_order_fulfillment(&mut self, data: &mut HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {

        for (key,v ) in data.iter_mut() {
            let mut done_orders = vec![];
            let v2 = v.clone();


            if !v.self_initiated_not_debug() {
                continue;
            }
            // if v.active_self().is_some() {
            //     continue;
            // }
            if let Some(ps) = v.party_events.as_ref() {
                // let key_address = key.address()?;
                let cutoff_time = current_time_millis_i64() - (self.relay.node_config.order_cutoff_delay_time().as_millis() as i64); //
                let orders = ps.orders();
                let cutoff_orders = ps.orders().iter().filter(|o| o.event_time < cutoff_time).cloned().collect_vec();
                // let identifier = v.party_info.initiate.safe_get()?.identifier.safe_get().cloned()?;

                self.metrics(
                    key.clone(), v.clone(), ps.clone(), orders.clone(), cutoff_orders.clone()
                ).await?;

                if cutoff_orders.len() > 0 {
                    info!("Party {} has orders: {}", key.hex(), cutoff_orders.len());

                    for (i,o) in ps.events.iter().enumerate() {
                        let ident = o.identifier();
                        let t = o.internal_external_str();
                        let inc = o.incoming();
                        let other = o.other_swap_address().unwrap_or("".to_string());
                        info!("Party event {i} type:{t} ident {ident}  inc {inc} other {other}")
                    }

                    for (i,o) in cutoff_orders.iter().enumerate() {
                        let event_id = o.primary_event.identifier();
                        let other = o.primary_event.other_swap_address().unwrap();
                        let destination = o.destination.render_string().unwrap();
                        let cur = o.order_amount_typed.currency_or().abbreviated();
                        let ocur = o.destination.currency_or().abbreviated();
                        let am = o.fulfilled_amount_typed.to_fractional();
                        let iam = o.order_amount_typed.to_fractional();
                        info!("Order {i} from {cur} to {ocur} with amount {iam} to {destination} fulfilled amount {am} event {event_id} other {other}");

                    }

                    let o2 = cutoff_orders.clone();
                    let grouped = o2.iter()
                        .group_by(|o| o.destination.currency_or());

                    // Convert groups into owned Vec before processing
                    let groups: Vec<(SupportedCurrency, Vec<&OrderFulfillment>)> = grouped
                        .into_iter()
                        .map(|(key, group)| (key, group.collect()))
                        .collect();

                    for (cur, group) in groups {
                        let mut o = vec![];
                        for i in group {
                            let dest = i.destination.clone();
                            let amt = i.fulfilled_amount_typed.clone();
                            o.push((dest, amt, i));
                        }


                        let amm_addr = ps.address_for_currency(&cur).ok_msg("Missing address for currency")?;
                        let all_members = v2.metadata.members_of(&amm_addr);
                        let peers = all_members.clone()
                            .into_iter().filter(|pk| pk != key)
                            .collect_vec();
                        let option = v2.metadata.instance_of_address(&amm_addr).ok_msg("Missing instances of address")?;
                        let thresh = option.threshold.as_ref().ok_msg("Missing threshold")?;
                        let threshold = thresh.value;
                        let descriptor = AddressDescriptor::from_multisig_public_keys_and_threshold(&all_members, threshold);
                        if cur == SupportedCurrency::Redgold {
                            for (dest, amt, o) in o {
                                let mut builder = TransactionBuilder::new(&self.relay.node_config);
                                let mut wrapper = builder
                                    .with_input_address_descriptor(&descriptor);
                                let utxos = self.relay.ds.transaction_store.query_utxo_address(&amm_addr).await?;
                                let mut b = wrapper
                                    .with_utxos(&utxos)?;
                                b.with_output(&dest, &amt);
                                if let Some(u) = o.stake_withdrawal_fulfilment_utxo_id.as_ref() {
                                    b.with_last_output_stake_withdrawal_fulfillment(u)?;
                                } else if let Some(txid) = o.tx_id_ref.as_ref() {
                                    b.with_last_output_deposit_swap_fulfillment(txid.clone())?;
                                }
                                info!("Sending multisig tx to destination {} with amount {}", dest.render_string().unwrap(), amt.to_fractional());
                                // info!("Building multisig tx for rdg party aaddress: {}", rdg_address.json_or());
                                // info!("Building multisig tx for rdg party aaddress amm addr: {}", amm_addr.json_or());
                                let orig_tx = b.build()?.sign_multisig(&self.relay.node_config.keypair(), &amm_addr)?;

                                let mut req = Request::default();
                                let mut mreq = MultisigRequest::default();
                                mreq.proposer_party_key = Some(key.clone());
                                mreq.destination = Some(dest.clone());
                                mreq.amount = Some(amt.clone());
                                mreq.mp_address = Some(amm_addr.clone());
                                mreq.currency = cur as i32;
                                mreq.tx = Some(orig_tx.clone());
                                req.multisig_request = Some(mreq);
                                let responses = self.relay.broadcast_async(peers.clone(), req, None).await?;
                                let mut merged_tx = orig_tx.clone();
                                let mut valid_peer_responses = 0;
                                for r in responses.into_iter() {
                                    if let Ok(tx) = r
                                        .and_then(|r| r.multisig_response.clone().ok_msg("Missing multisig response"))
                                        .and_then(|r| r.tx.clone().ok_msg("Missing tx")).log_error() {
                                        merged_tx = merged_tx.combine_multisig_proofs(&tx, &amm_addr)?;
                                        valid_peer_responses += 1;
                                    }
                                }
                                let met_thresh = merged_tx.inputs.iter()
                                    .find(|x| x.proof.len() >= threshold as usize)
                                    .is_some();
                                let input_proof_len = merged_tx.inputs.iter().map(|x| x.proof.len()).next().unwrap_or(0);
                                if !met_thresh {
                                    error!(
                                        "Failed to meet threshold for multisig tx: {} out of {} and {} peer responses",
                                        input_proof_len,
                                        threshold,
                                        valid_peer_responses
                                    );
                                    error!("Broke here");
                                } else {
                                    info!("Submitting multisig tx for rdg: {}", merged_tx.hash_hex());
                                    let submitted = self.relay.submit_transaction_sync(&merged_tx).await.log_error();
                                    if submitted.is_ok() {
                                        done_orders.push(o.clone());
                                    }
                                }
                            }
                        } else { // if cur.only_one_destination_per_tx()
                            for (dest, amt, o) in o {
                                let result = self.external_network_resources
                                    .execute_external_multisig_send(
                                        vec![(dest.clone(), amt.clone())], &amm_addr,
                                        &peers,
                                        &self.relay,
                                        threshold
                                    ).await.log_error();
                                if result.is_ok() {
                                    done_orders.push(o.clone());
                                }
                                info!("Sent multisig external tx to destination {} with amount {} and txid {}",
                                    dest.render_string().unwrap(), amt.to_fractional(), result.ok().map(|x| x.identifier).unwrap_or("".to_string())
                                );
                            }
                        }
                        // else {
                        //     self.external_network_resources
                        //         .prepare_multisig(o.into_iter().map(|(dest, amt, _o)| (dest, amt)).collect(), &amm_addr).await?;
                        // }
                    }

                }
                // let mut done_orders = vec![];
                // let btc = self.fulfill_btc_orders(key, &identifier, ps, cutoff_time).await.log_error().ok();
                // if let Some(b) = btc {
                //     done_orders.extend(b);
                // }
                // let eth = self.fulfill_eth(ps, &identifier, v2).await.log_error().ok();
                // if let Some(e) = eth {
                //     done_orders.extend(e);
                // }
                //
                // let mut total_done_orders = done_orders.len();
                //
                // ps.process_locally_fulfilled_orders(done_orders);
                if let Some(lfo) = v.locally_fulfilled_orders.as_mut() {
                    lfo.extend(done_orders);
                } else {
                    v.locally_fulfilled_orders = Some(done_orders.clone());
                }
                //
                // Immediately update processed orders ^ to ensure no duplicate or no persistence failure
               
                // let rdg_fulfilled = self.fulfill_rdg_orders(&identifier, &utxos, ps, cutoff_time).await?;
                // total_done_orders += rdg_fulfilled;
                // gauge!("redgold_party_fulfilled_orders_now", &pk_label).set(total_done_orders as f64);

            }
        }
        Ok(())
    }

    async fn metrics(&self, key: PublicKey, v: PartyInternalData, ps: PartyEvents, orders: Vec<OrderFulfillment>, cutoff_orders: Vec<OrderFulfillment>) -> Result<(), ErrorInfo> {
        let rdg_address = ps.address_for_currency(&SupportedCurrency::Redgold);
        let balance = if let Some(a) = rdg_address.as_ref() {
            self.relay.ds.transaction_store.get_balance(a).await?
        } else {
            None
        };
        let rdg_ds_balance: i64 = balance.safe_get_msg("Missing balance").cloned().unwrap_or(0);

        let num_events = ps.events.len();
        let num_unconfirmed = ps.unconfirmed_events.len();
        let num_unfulfilled_deposits = ps.unfulfilled_incoming_external_amount_to_outgoing_rdg_orders.len();
        let num_unfulfilled_withdrawals = ps.unfulfilled_internal_tx_requiring_external_outgoing_mpc_withdrawals.len();
        let utxos = if let Some(a) = rdg_address.as_ref() {
            self.relay.ds.transaction_store.query_utxo_address(&a).await?
        } else {
            vec![]
        };

        let num_pending_stake_deposits = ps.pending_external_staking_txs.len();
        let fulfilled = ps.fulfillment_history.len();
        let internal_staking_events = ps.internal_staking_events.len();
        let external_staking_events = ps.external_staking_events.len();
        let rejected_stake_withdrawals = ps.rejected_stake_withdrawals.len();
        let num_internal_events = ps.num_internal_events();
        let num_external_events = ps.num_external_events();
        let num_external_incoming = ps.num_external_incoming_events();

        let party_pk_hex = key.hex();
        let party_pk_hex2 = key.hex();

        let kv_label = |k: String, v: String| {
            [("party_key".to_string(), party_pk_hex2.clone()), (k, v)]
        };
        let cur_label = |v: SupportedCurrency| {
            kv_label("currency".to_string(), v.abbreviated().to_string())
        };
        let pk_label = [("party_key".to_string(), party_pk_hex)];

        gauge!("redgold_ds_party_balance", &cur_label(SupportedCurrency::Redgold)).set(CurrencyAmount::from(rdg_ds_balance).to_fractional());
        for (k, v) in ps.balance_map.iter() {
            gauge!("redgold_party_stream_balance", &cur_label(k.clone())).set(v.to_fractional());
        }
        for (k, v) in ps.balance_with_deltas_applied.iter() {
            gauge!("redgold_party_stream_balance_with_deltas", &cur_label(k.clone())).set(v.to_fractional());
        }
        for (k, v) in ps.balance_pending_order_deltas_map.iter() {
            gauge!("redgold_party_stream_balance_pending_order_deltas", &cur_label(k.clone())).set(v.to_fractional());
        }
        for (k, v) in ps.balances_with_deltas_sub_portfolio().iter() {
            gauge!("redgold_party_stream_balance_sub_portfolio", &cur_label(k.clone())).set(v.to_fractional());
        }
        for (k, v) in ps.staking_balances(&vec![], Some(true), Some(true)).iter() {
            gauge!("redgold_party_stream_staking_balance", &cur_label(k.clone())).set(v.to_fractional());
        }
        for (k, v) in ps.event_counts() {
            gauge!("redgold_party_stream_event_counts", &cur_label(k.clone())).set(v as f64);
        }
        gauge!("redgold_party_stream_total_events", &pk_label).set(num_events as f64);
        gauge!("redgold_party_num_unconfirmed", &pk_label).set(num_unconfirmed as f64);
        gauge!("redgold_party_num_unfulfilled_deposits", &pk_label).set(num_unfulfilled_deposits as f64);
        gauge!("redgold_party_num_unfulfilled_withdrawals", &pk_label).set(num_unfulfilled_withdrawals as f64);
        gauge!("redgold_party_num_utxos", &pk_label).set(utxos.len() as f64);
        gauge!("redgold_party_num_pending_stake_deposits", &pk_label).set(num_pending_stake_deposits as f64);
        gauge!("redgold_party_fulfilled", &pk_label).set(fulfilled as f64);
        gauge!("redgold_party_internal_staking_events", &pk_label).set(internal_staking_events as f64);
        gauge!("redgold_party_external_staking_events", &pk_label).set(external_staking_events as f64);
        gauge!("redgold_party_rejected_stake_withdrawals", &pk_label).set(rejected_stake_withdrawals as f64);
        gauge!("redgold_party_num_internal_events", &pk_label).set(num_internal_events as f64);
        gauge!("redgold_party_num_external_events", &pk_label).set(num_external_events as f64);
        gauge!("redgold_party_num_external_incoming", &pk_label).set(num_external_incoming as f64);
        gauge!("redgold_party_cutoff_orders", &pk_label).set(cutoff_orders.len() as f64);
        gauge!("redgold_party_orders", &pk_label).set(orders.len() as f64);
        gauge!("redgold_party_locally_fulfilled_orders,", &pk_label).set(ps.locally_fulfilled_orders.len() as f64);
        for (k, v) in v.network_data.iter() {
            gauge!("redgold_party_num_network_tx", &cur_label(k.clone())).set(v.transactions.len() as f64);
        }

        for (k, c) in ps.central_prices.iter() {
            gauge!("redgold_party_central_price_min_ask_estimated", &cur_label(k.clone())).set(c.min_ask_estimated);
            gauge!("redgold_party_central_price_min_bid_estimated", &cur_label(k.clone())).set(c.min_bid_estimated);
        }
        for (k, v) in ps.event_counts() {
            gauge!("redgold_party_stream_event_counts", &cur_label(k.clone())).set(v as f64);
        }
        for (k, v) in ps.portfolio_request_events.current_portfolio_imbalance.iter() {
            gauge!("redgold_party_portfolio_imbalance", &cur_label(k.clone())).set(v.to_fractional());
        }

        for (k, v) in ps.portfolio_request_events.external_stake_balance_deltas.iter() {
            gauge!("redgold_party_portfolio_external_stake_balance_deltas", &cur_label(k.clone())).set(v.to_fractional());
        }
        gauge!("redgold_party_portfolio_stake_utxos", &pk_label).set(ps.portfolio_request_events.stake_utxos.len() as f64);

        for (k, v) in ps.portfolio_request_events.current_rdg_allocations.iter() {
            gauge!("redgold_party_portfolio_rdg_allocations", &cur_label(k.clone())).set(v.to_fractional());
        }
        Ok(())
    }


}