use bitcoin::secp256k1::rand;
use bitcoin::secp256k1::rand::Rng;
use crate::{Address, constants, ErrorInfo, KeyPair, NetworkEnvironment, PeerData, SafeOption, struct_metadata_new, StructMetadata, Transaction, util};
use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY};
use crate::structs::{AddressInfo, NodeMetadata, Output, Proof, StandardData, TransactionAmount, TransactionData, TransactionOptions, UtxoEntry};
use crate::transaction::amount_data;

pub struct TransactionBuilder {
    pub transaction: Transaction,
    pub balance: i64,
    pub utxos: Vec<UtxoEntry>,
    pub used_utxos: Vec<UtxoEntry>
}

impl TransactionBuilder {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            transaction: Transaction{
                inputs: vec![],
                outputs: vec![],
                struct_metadata: struct_metadata_new(),
                options: Some(TransactionOptions{
                    salt: Some(rng.gen::<i64>()),
                    // TODO: None here or with setter?
                    network_type: None,
                    key_value_options: vec![],
                    data: None,
                    contract: None,
                    offline_time_sponsor: None,
                }),
            },
            balance: 0,
            utxos: vec![],
            used_utxos: vec![],
        }
    }

    pub fn with_utxo(&mut self, utxo_entry: &UtxoEntry) -> Result<&mut Self, ErrorInfo> {
        let entry = utxo_entry.clone();
        let o = utxo_entry.output.safe_get_msg("Missing output")?;
        o.address.safe_get_msg("Missing address")?;
        // TODO: This will throw errors on any input not currency related, be warned
        // Or use separate method to deal with data inputs?
        o.safe_ensure_amount()?;
        self.utxos.push(entry);
        Ok(self)
    }

    pub fn with_maybe_currency_utxo(&mut self, utxo_entry: &UtxoEntry) -> Result<&mut Self, ErrorInfo> {
        let o = utxo_entry.output.safe_get_msg("Missing output")?;
        if let Ok(a) = o.safe_ensure_amount() {
            self.with_utxo(utxo_entry)?;
        }
        Ok(self)
    }

    pub fn with_message(&mut self, msg: impl Into<String>) -> Result<&mut Self, ErrorInfo> {
        let mut x = self.transaction.options.as_mut().expect("");
        match x.data.as_mut() {
            None => {
                let mut data = TransactionData::default();
                data.message = Some(msg.into());
                x.data = Some(data);
            }
            Some(d) => {
                d.message = Some(msg.into());
            }
        }
        Ok(self)
    }

    pub fn with_address_info(&mut self, ai: AddressInfo) -> Result<&mut Self, ErrorInfo> {
        for u in ai.utxo_entries {
            self.with_utxo(&u)?;
        }
        Ok(self)
    }

    pub fn with_output(&mut self, destination: &Address, amount: &TransactionAmount) -> &mut Self {
        self.balance -= amount.amount;
        let output = Output {
            address: Some(destination.clone()),
            data: amount_data(amount.amount as u64),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_output_all(&mut self, destination: &Address) -> &mut Self {
        let output = Output {
            address: Some(destination.clone()),
            data: amount_data(self.balance as u64),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.balance = 0;
        self.transaction.outputs.push(output);
        self
    }

    // Should this be hex or bytes data?
    pub fn with_last_output_deposit_swap(&mut self, btc_txid: String) -> &mut Self {
        if let Some(o) = self.transaction.outputs.last_mut() {
            if let Some(d) = o.data.as_mut() {
                d.bitcoin_txid = Some(btc_txid)
            }
        }
        self
    }

        // Aha heres the issue, we're expecting output to be populated here
    // Remove this assumption elsewhere
    pub fn with_unsigned_input(&mut self, utxo: UtxoEntry) -> Result<&mut Self, ErrorInfo> {
        let mut input = utxo.to_input();

        let output = utxo.output.safe_get()?;
        let data = output.data.safe_get()?;
        if let Some(a) = data.amount {
            self.balance += a;
        }
        self.used_utxos.push(utxo.clone());
        self.transaction.inputs.push(input);
        Ok(self)
    }

    pub fn build(&mut self) -> Result<Transaction, ErrorInfo> {

        // TODO: Should we sort the other way instead? To compress as many UTXOs as possible?
        // In order for that we'd need to raise transaction size limit
        self.utxos.sort_by(|a, b| a.amount().cmp(&b.amount()));

        for u in self.utxos.clone() {
            self.with_unsigned_input(u.clone())?;
            if self.balance > 0 {
                break
            }
        }
        if self.balance < 0 {
            return Err(ErrorInfo::error_info("Insufficient funds"));
        }

        if self.balance > 0 {
            self.with_remainder();
        }

        Ok(self.transaction.clone())
        // self.balance
    }

    pub fn with_output_peer_data(&mut self, destination: &Address, pd: PeerData) -> &mut Self {
        let output = Output {
            address: Some(destination.clone()),
            data: StandardData::peer_data(pd),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_output_node_metadata(&mut self, destination: &Address, pd: NodeMetadata) -> &mut Self {
        let mut data = StandardData::default();
        data.node_metadata = Some(pd);
        let data = Some(data);
        let output = Output {
            address: Some(destination.clone()),
            data,
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_remainder(&mut self) -> &mut Self {
        let address = self
            .transaction.inputs.get(0)
            .expect("missing head")
            .output
            .as_ref()
            .expect("o")
            .address
            .as_ref()
            .expect("address")
            .clone();

        let output = Output {
            address: Some(address),
            data: amount_data(self.balance as u64),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

}