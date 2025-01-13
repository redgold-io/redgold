use std::collections::HashMap;
use redgold_common_no_wasm::arc_swap_wrapper::WriteOneReadAll;
use redgold_schema::RgResult;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;

#[derive(Clone, Default)]
pub struct ExternalDaq {
    pub network: NetworkEnvironment,
    pub subscribed_address_filter: WriteOneReadAll<Vec<String>>,
    pub recent_transactions: WriteOneReadAll<HashMap<String, Vec<ExternalTimedTransaction>>>,
    pub historical_transactions: WriteOneReadAll<HashMap<String, RgResult<Vec<ExternalTimedTransaction>>>>
}

impl ExternalDaq {
    pub fn add_new_tx(&mut self, tx: ExternalTimedTransaction) {
        let mut recent_tx = (*self.recent_transactions.read()).clone();
        let txs = recent_tx.entry(tx.other_address.clone()).or_insert(vec![]);
        txs.push(tx.clone());
        self.recent_transactions.write(recent_tx);
    }

    pub fn add_historical_backfill(&mut self, a: &String, mut txs: RgResult<Vec<ExternalTimedTransaction>>) {

        let recent = self.recent_transactions.read();
        let by_a = recent.get(a);
        if let Some(recent_txs) = by_a {
            txs = match txs {
                Ok(mut these_tx) => {
                    these_tx.retain(|t| recent_txs.iter().all(|rt| rt.tx_id != t.tx_id));
                    Ok(these_tx)
                }
                Err(e) => { Err(e) }
            }

        }

        let mut historical_tx = (*self.historical_transactions.read()).clone();
        historical_tx.insert(a.clone(), txs);
        self.historical_transactions.write(historical_tx);
    }

}