use redgold_schema::structs::{Address, CurrencyAmount, DepositRequest, StakeDeposit, StakeWithdrawal, SupportedCurrency, Transaction, UtxoId};
use redgold_schema::RgResult;
use num_bigint::BigInt;
use itertools::Itertools;
use rocket::form::validate::Contains;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::proof_support::PublicKeySupport;
use rocket::serde::{Deserialize, Serialize};
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
                let matching = self.external_unfulfilled_staking_txs.iter()
                    .filter(|s| s.amount == amt && s.external_address == addr)
                    .next()
                    .cloned();
                if let Some(m) = matching {
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

    pub fn minimum_stake_amount(amt: &CurrencyAmount) -> bool {
        match amt.currency_or() {
            SupportedCurrency::Redgold => {
                amt.to_fractional() >= 1.0
            }
            SupportedCurrency::Bitcoin => {
                amt.amount >= 10000
            }
            SupportedCurrency::Ethereum => {
                amt.bigint_amount().map(|b| b >= BigInt::from(1e11 as i64)).unwrap_or(false)
            }
            _ => false
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
                    self.external_unfulfilled_staking_txs.push(PendingExternalStakeEvent {
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

    pub(crate) fn handle_stake_requests(&mut self, event: &AddressEvent, time: i64, tx: &Transaction) -> RgResult<()> {
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
                self.process_withdrawal(event, tx, withdrawal);
            }
        }
        Ok(())
    }

    fn process_withdrawal(&mut self, event: &AddressEvent, tx: &Transaction, withdrawal: &StakeWithdrawal) {
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
                self.pending_stake_withdrawals.push(WithdrawalStakingEvent {
                    address: d.clone(),
                    amount: amt,
                    initiating_event: event.clone(),
                });
            }
        }
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
            if amt.currency() == SupportedCurrency::Redgold && Self::minimum_stake_amount(&amt) {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
pub struct WithdrawalStakingEvent {
    pub address: Address,
    pub amount: CurrencyAmount,
    pub initiating_event: AddressEvent,
}
