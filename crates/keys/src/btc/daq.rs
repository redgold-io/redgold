use std::str::FromStr;
use bdk::bitcoin::{Address, Network, TxIn, TxOut};
use bdk::blockchain::GetTx;
use bdk::database::BatchDatabase;
use bdk::{Balance, TransactionDetails};
use itertools::Itertools;
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use crate::btc::btc_wallet::SingleKeyBitcoinWallet;

impl<D: BatchDatabase> SingleKeyBitcoinWallet<D> {

    pub fn parse_address(addr: &String) -> RgResult<Address> {
        Address::from_str(&addr).error_info("Unable to convert destination pk to bdk address")
    }


    pub fn convert_network(network_environment: &NetworkEnvironment) -> Network {
        if *network_environment == NetworkEnvironment::Main {
            Network::Bitcoin
        } else {
            Network::Testnet
        }
    }

    pub fn outputs_convert(&self, outs: &Vec<TxOut>) -> Vec<(String, u64)> {
        let mut res = vec![];
        for o in outs {
            let a = Address::from_script(&o.script_pubkey, self.network).ok();
            if let Some(a) = a {
                res.push((a.to_string(), o.value))
            }
        }
        res
    }

    pub fn outputs_convert_static(outs: &Vec<TxOut>, network: NetworkEnvironment) -> Vec<(String, u64)> {
        let mut res = vec![];
        for o in outs {
            let a = Address::from_script(&o.script_pubkey, Self::convert_network(&network)).ok();
            if let Some(a) = a {
                res.push((a.to_string(), o.value))
            }
        }
        res
    }

    pub fn convert_tx_inputs_address(&self, tx_ins: &Vec<TxIn>) -> RgResult<Vec<(String, u64)>> {
        let mut res = vec![];
        for i in tx_ins {
            let txid = i.previous_output.txid;
            let vout = i.previous_output.vout;
            let prev_tx = self.client.get_tx(&txid).error_info("Error getting tx")?;
            let prev_tx = prev_tx.safe_get_msg("No tx found")?;
            let prev_output = prev_tx.output.get(vout as usize);
            let prev_output = prev_output.safe_get_msg("Error getting output")?;
            let amount = prev_output.value;
            let a = Address::from_script(&prev_output.script_pubkey, self.network).ok();
            // println!("{}", format!("TxIn address: {:?}", a));
            if let Some(a) = a {
                let a = a.to_string();
                res.push((a, amount));
            }
        }
        Ok(res)
    }
    pub fn get_all_tx(&self) -> Result<Vec<ExternalTimedTransaction>, ErrorInfo> {
        let mut res = vec![];
        let result = self.wallet.list_transactions(true)
            .error_info("Error listing transactions")?;
        for tranaction_details in result.iter() {
            let ett = self.extract_ett(tranaction_details)?;
            if let Some(ett) = ett {
                res.push(ett)
            }
        }
        Ok(res)
    }

    pub fn extract_ett(&self, transaction_details: &TransactionDetails) -> Result<Option<ExternalTimedTransaction>, ErrorInfo> {
        let self_addr = self.address()?;

        let tx = transaction_details.transaction.safe_get_msg("Error getting transaction")?;
        let output_amounts = self.outputs_convert(&tx.output);
        let other_output_addresses = output_amounts.iter().filter_map(|(output_address, _y)| {
            if output_address != &self_addr {
                Some(output_address.clone())
            } else {
                None
            }
        }).collect();
        let input_addrs = self.convert_tx_inputs_address(&tx.input)?;

        // Not needed?
        // let has_self_output = output_amounts.iter().filter(|(x,y)| x != &self_addr).next().is_some();
        let has_self_input = input_addrs.iter().filter(|(x, _y)| x == &self_addr).next().is_some();
        let incoming = !has_self_input;

        let other_address = if incoming {
            input_addrs.iter().filter(|(x, _y)| x != &self_addr).next().map(|(x, _y)| x.clone())
        } else {
            output_amounts.iter().filter(|(x, _y)| x != &self_addr).next().map(|(x, _y)| x.clone())
        };

        let from =
            input_addrs.iter().next().map(|(x, _y)| structs::Address::from_bitcoin_external(x))
                .ok_msg("No input address found")?;

        let to = output_amounts.iter().map(|(x, y)| {
            (structs::Address::from_bitcoin_external(x), CurrencyAmount::from_btc(*y as i64))
        }).collect_vec();

        let amount = if incoming {
            output_amounts.iter().filter(|(x, _y)| x == &self_addr).next().map(|(_x, y)| y.clone())
        } else {
            output_amounts.iter().filter(|(x, _y)| x != &self_addr).next().map(|(_x, y)| y.clone())
        };

        let block_timestamp = transaction_details.confirmation_time.clone().map(|x| x.timestamp).map(|t| t * 1000);
        let fee = transaction_details.fee.map(|f| CurrencyAmount::from_btc(f as i64));
        let ett = if let (Some(a), Some(value)) = (other_address, amount) {
            Some(ExternalTimedTransaction {
                tx_id: transaction_details.txid.to_string(),
                timestamp: block_timestamp,
                other_address: a.clone(),
                other_output_addresses,
                amount: value,
                bigint_amount: None,
                incoming,
                currency: SupportedCurrency::Bitcoin,
                block_number: None,
                price_usd: None,
                fee,
                self_address: Some(self_addr),
                currency_id: Some(SupportedCurrency::Bitcoin.into()),
                currency_amount: Some(CurrencyAmount::from_btc(value as i64)),
                from: from,
                to: to,
                other: Some(structs::Address::from_bitcoin_external(&a)),
            })
        } else {
            None
        };
        Ok(ett)
    }

    pub fn get_wallet_balance(&self
    ) -> Result<Balance, ErrorInfo> {
        self.sync()?;
        let balance = self.wallet.get_balance().error_info("Error getting BDK wallet balance")?;
        Ok(balance)
    }

    pub fn balance(&self) -> RgResult<CurrencyAmount> {
        let c = self.get_wallet_balance()?.confirmed;
        Ok(CurrencyAmount::from_btc(c as i64))
    }

}