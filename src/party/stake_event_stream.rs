use redgold_schema::structs::{Address, CurrencyAmount, DepositRequest, StakeDeposit, StakeWithdrawal, SupportedCurrency, Transaction, UtxoId};
use redgold_schema::RgResult;
use num_bigint::BigInt;
use itertools::Itertools;
use rocket::form::validate::{Contains, with};
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::proof_support::PublicKeySupport;
use rocket::serde::{Deserialize, Serialize};
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::eth::historical_client::EthHistoricalClient;
use redgold_keys::util::btc_wallet::ExternalTimedTransaction;
use crate::party::address_event::AddressEvent;
use crate::party::party_stream::PartyEvents;

impl PartyEvents {
    //

    pub fn check_external_event_pending_stake(&mut self, event_o: &AddressEvent) -> bool {
        if let AddressEvent::External(event) = event_o {
            let amt = event.currency_amount();
            let oa = event.other_address_typed();
            if let Ok(addr) = oa {
                let matching = self.pending_external_staking_txs.iter()
                    .filter(|s| s.amount == amt && s.external_address == addr)
                    .next()
                    .cloned();
                if let Some(m) = matching {
                    self.pending_external_staking_txs.retain(|s| s.utxo_id != m.utxo_id);
                    let ev = ConfirmedExternalStakeEvent {
                        pending_event: m,
                        event: event_o.clone(),
                    };
                    self.external_staking_events.push(ev);
                    return true;
                }
            }
        }
        false
    }

    pub fn meets_minimum_stake_amount(amt: &CurrencyAmount) -> bool {
        Self::minimum_stake_amount_total(amt.currency_or())
            .map(|min| amt.clone() >= min)
            .unwrap_or(false)
    }

    pub fn minimum_stake_amount_total(currency: SupportedCurrency) -> Option<CurrencyAmount> {
        match currency {
            SupportedCurrency::Redgold => {
                Some(CurrencyAmount::from_fractional(1.0).unwrap())
            }
            SupportedCurrency::Bitcoin => {
                Some(CurrencyAmount::from_btc(10_000))
            }
            SupportedCurrency::Ethereum => {
                Some(CurrencyAmount::from_eth_fractional(0.005))
            }
            _ => None
        }
    }

    pub fn expected_fee_amount(currency: SupportedCurrency) -> Option<CurrencyAmount> {
        match currency {
            SupportedCurrency::Redgold => {
                Some(CurrencyAmount::from_fractional(0.0001).unwrap())
            }
            SupportedCurrency::Bitcoin => {
                Some(CurrencyAmount::from_btc(2_000))
            }
            SupportedCurrency::Ethereum => {
                Some(EthWalletWrapper::fee_fixed_normal())
            }
            _ => None
        }
    }

    fn handle_external_liquidity_deposit(
        &mut self, event: &AddressEvent, tx: &Transaction, deposit_inner: &DepositRequest, liquidity_deposit: &StakeDeposit,
        utxo_id: UtxoId) {
        if let Some(amt) = deposit_inner.amount.as_ref() {
            let pk_first = tx.first_input_proof_public_key();
            if let Some(pk) = pk_first {
                if let Some(addr) = match amt.currency_or() {
                    SupportedCurrency::Bitcoin => {
                        pk.to_bitcoin_address_typed(&self.network).ok()
                    }
                    SupportedCurrency::Ethereum => {
                        pk.to_ethereum_address_typed().ok()
                    }
                    _ => None
                } {
                    self.pending_external_staking_txs.push(PendingExternalStakeEvent {
                        event: event.clone(),
                        tx: tx.clone(),
                        amount: amt.clone(),
                        external_address: addr.clone(),
                        external_currency: amt.currency_or(),
                        liquidity_deposit: liquidity_deposit.clone(),
                        deposit_inner: deposit_inner.clone(),
                        utxo_id
                    })
                }
            }
        }
    }

    pub fn handle_stake_requests(&mut self, event: &AddressEvent, time: i64, tx: &Transaction) -> RgResult<()> {
        let addrs = self.party_public_key.to_all_addresses()?;
        let amt = Some(addrs.iter().map(|a| tx.output_rdg_amount_of(a)).sum::<i64>())
            .filter(|a| *a > 0)
            .map(|a| CurrencyAmount::from(a));
        let opt_stake_request_utxo_id = tx.stake_requests();
        // let opt_stake_request_utxo_id = addrs.iter().flat_map(|a| tx.liquidity_of(a)).next();
        for ((utxo_id, req)) in opt_stake_request_utxo_id {
            if let Some(deposit) = req.deposit.as_ref() {
                // This represents an external deposit.
                if let Some(deposit_inner) = deposit.deposit.as_ref() {
                    // deposit_inner.address
                    self.handle_external_liquidity_deposit(event, tx, deposit_inner, deposit, utxo_id.clone());
                } else {
                    // Mark this as an internal staking event with some balance due to the amount being present
                    self.internal_liquidity_stake(event, tx, amt.clone(), deposit, utxo_id.clone());
                }
            } else if let Some(withdrawal) = req.withdrawal.as_ref() {
                self.process_stake_withdrawal(event, tx, withdrawal, time, utxo_id.clone())?;
            }
        }
        Ok(())
    }

    pub fn process_stake_withdrawal(&mut self, event: &AddressEvent, tx: &Transaction, withdrawal: &StakeWithdrawal, time: i64, id: UtxoId) -> RgResult<()> {
        let input_utxo_ids = tx.input_utxo_ids().collect_vec();
        // Find inputs corresponding to staking events.
        // This represents a withdrawal, either external or internal

        if let Some(d) = withdrawal.destination.as_ref() {
            let w_currency = d.currency_or();
            let amount = {
                if w_currency == SupportedCurrency::Redgold {
                    if let Some(ev) = self.internal_staking_events.iter()
                        .filter(|s| input_utxo_ids.contains(&s.utxo_id))
                        .next().cloned() {
                        self.internal_staking_events.retain(|s| s.utxo_id != ev.utxo_id);
                        Some(ev.amount.clone())
                    } else {
                        None
                    }
                } else {
                    self.retain_external_stake(&input_utxo_ids, w_currency)
                }
            };
            if let Some(amt) = amount {
                if let Some(existing) = self.balance_map.get(&amt.currency_or()) {
                    let minimum_amt = Self::minimum_stake_amount_total(
                        amt.currency_or()).unwrap_or(CurrencyAmount::zero(amt.currency_or())
                    );
                    if let Some(fee) = Self::expected_fee_amount(amt.currency_or()) {
                        let expected_fee = fee.clone();
                        let delta = existing.clone() - amt.clone() - minimum_amt.clone() - (expected_fee.clone() * 2);
                        let order_amt = if delta > minimum_amt.clone() {
                            let candidates = vec![delta, amt];
                            let min = candidates.iter().min().expect("min").clone();
                            Some(min)
                        } else if !self.network.is_main() {
                            let reduced = existing.clone() - (fee.clone() * 2);
                            if reduced > fee {
                                Some(reduced)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(order_amt) = order_amt {
                            self.fulfill_order(order_amt.clone(), false, time, None, &d, true, event, Some(id.clone()))?;
                        }
                    }
                }
            }
        }
        if self.event_fulfillment.is_none() {
            self.rejected_stake_withdrawals.push(event.clone());
        }
        if let Some(of) = self.event_fulfillment.clone() {
            let is_external = of.is_stake_withdrawal && of.destination.currency_or() != SupportedCurrency::Redgold;
            // Is this even being used anymore?
            let w = PendingWithdrawalStakeEvent {
                address: withdrawal.destination.clone().expect("destination"),
                amount: of.fulfilled_currency_amount(),
                initiating_event: event.clone(),
                is_external,
                utxo_id: id
            };
            self.pending_stake_withdrawals.push(w);
            if is_external {
                self.unfulfilled_external_withdrawals.push((of, event.clone()));
            } else {
                self.unfulfilled_rdg_orders.push((of, event.clone()));
            }
        }
        Ok(())
    }

    fn retain_external_stake(&mut self, utxo_ids: &Vec<&UtxoId>, w_currency: SupportedCurrency) -> Option<CurrencyAmount> {
        if let Some(ev) = self.external_staking_events.iter()
            .filter(|s| utxo_ids.contains(&s.pending_event.utxo_id) &&
                s.pending_event.external_currency == w_currency)
            .next().cloned() {
            self.external_staking_events.retain(|s| s.pending_event.utxo_id != ev.pending_event.utxo_id);
            Some(ev.pending_event.amount.clone())
        } else {
            None
        }
    }

    fn internal_liquidity_stake(&mut self, event: &AddressEvent, tx: &Transaction, amt: Option<CurrencyAmount>, deposit: &StakeDeposit, utxo_id: UtxoId) {
        if let Some(amt) = amt.clone() {
            if amt.currency() == SupportedCurrency::Redgold && Self::meets_minimum_stake_amount(&amt) {
                if let Some(withdrawal_address) = tx.first_input_proof_public_key()
                    .and_then(|pk| pk.address().ok()) {
                    self.internal_staking_events.push(InternalStakeEvent {
                        event: event.clone(),
                        tx: tx.clone(),
                        amount: amt,
                        withdrawal_address,
                        liquidity_deposit: deposit.clone(),
                        utxo_id
                    });
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InternalStakeEvent {
    pub event: AddressEvent,
    pub tx: Transaction,
    pub amount: CurrencyAmount,
    pub withdrawal_address: Address,
    pub liquidity_deposit: StakeDeposit,
    pub utxo_id: UtxoId,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingExternalStakeEvent {
    pub event: AddressEvent,
    pub tx: Transaction,
    pub amount: CurrencyAmount,
    pub external_address: Address,
    pub external_currency: SupportedCurrency,
    pub liquidity_deposit: StakeDeposit,
    pub deposit_inner: DepositRequest,
    pub utxo_id: UtxoId,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfirmedExternalStakeEvent {
    pub pending_event: PendingExternalStakeEvent,
    pub event: AddressEvent
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingWithdrawalStakeEvent {
    pub address: Address,
    pub amount: CurrencyAmount,
    pub initiating_event: AddressEvent,
    pub is_external: bool,
    pub utxo_id: UtxoId,
}
