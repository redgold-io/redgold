use crate::{Address, bytes_data, error_info, ErrorInfo, PeerData, RgResult, SafeOption, struct_metadata_new, Transaction};
use crate::structs::{AddressInfo, CodeExecutionContract, ExecutorBackend, FixedUtxoId, Input, NodeMetadata, Output, OutputContract, OutputType, StandardData, TransactionAmount, TransactionData, TransactionOptions, UtxoEntry};
use crate::transaction::amount_data;

pub struct TransactionBuilder {
    pub transaction: Transaction,
    pub utxos: Vec<UtxoEntry>,
    pub used_utxos: Vec<UtxoEntry>
}


impl TransactionBuilder {
    pub fn with_fee(&mut self, destination: &Address, amount: &TransactionAmount) -> RgResult<&mut Self> {
        self.with_output(destination, amount);
        let option = self.transaction.outputs.last_mut();
        let mut o = option.ok_or(error_info("Missing output"))?;
        o.output_type = Some(OutputType::Fee as i32);
        Ok(self)
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
        let output = Output::new(destination, amount.amount);
        self.transaction.outputs.push(output);
        self
    }



    pub fn with_contract_request_output(&mut self,
                                        destination: &Address,
                                        serialized_request: &Vec<u8>
    ) -> RgResult<&mut Self> {
        let mut o = Output::default();
        o.address = Some(destination.clone());
        let mut c = OutputContract::default();
        c.pay_update_descendents = true;
        o.contract = Some(c);
        let mut d = StandardData::default();
        d.request = bytes_data(serialized_request.clone());
        o.data = Some(d);
        o.output_type = Some(OutputType::RequestCall as i32);
        self.transaction.outputs.push(o);
        Ok(self)

    }

    // TODO: Do we need to deal with contract state here?
    pub fn with_contract_deploy_output_and_predicate_input(
        &mut self, code: impl AsRef<[u8]>, c_amount: TransactionAmount, use_predicate_input: bool) -> RgResult<&mut Self> {
        let destination = Address::script_hash(code.as_ref())?;
        let mut o = Output::default();
        o.address = Some(destination.clone());
        o.output_type = Some(OutputType::Deploy as i32);
        let mut contract = OutputContract::default();
        let mut code_exec = CodeExecutionContract::default();
        code_exec.code = bytes_data(code.as_ref().to_vec().clone());
        code_exec.executor = Some(ExecutorBackend::Extism as i32);
        contract.code_execution_contract = Some(code_exec);
        o.contract = Some(contract);
        o.data = amount_data(c_amount.amount as u64);
        self.transaction.outputs.push(o);
        if use_predicate_input {
            let input = Input::predicate_filter(&destination);
            self.transaction.inputs.push(input);
        }

        Ok(self)
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
        // if let Some(a) = data.amount {
        //     self.balance += a;
        // }
        self.used_utxos.push(utxo.clone());
        self.transaction.inputs.push(input);
        Ok(self)
    }

    pub fn balance(&self) -> i64 {
        self.transaction.total_input_amount() - self.transaction.total_output_amount()
    }

    pub fn build(&mut self) -> Result<Transaction, ErrorInfo> {

        // TODO: Should we sort the other way instead? To compress as many UTXOs as possible?
        // In order for that we'd need to raise transaction size limit
        self.utxos.sort_by(|a, b| a.amount().cmp(&b.amount()));

        for u in self.utxos.clone() {
            self.with_unsigned_input(u.clone())?;
            if self.balance() > 0 {
                break
            }
        }

        if self.balance() < 0 {
            return Err(ErrorInfo::error_info("Insufficient funds"));
        }

        if self.balance() > 0 {
            self.with_remainder();
        }

        Ok(self.transaction.clone())
        // self.balance
    }

    pub fn with_output_peer_data(&mut self, destination: &Address, pd: PeerData, height: i64) -> &mut Self {
        let mut option = StandardData::peer_data(pd).expect("o");
        option.height = Some(height);
        let mut output = Output::default();
        output.address = Some(destination.clone());
        output.data = Some(option);
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_output_node_metadata(&mut self, destination: &Address, pd: NodeMetadata, height: i64) -> &mut Self {
        let mut data = StandardData::default();
        data.node_metadata = Some(pd);
        data.height = Some(height);
        let data = Some(data);
        let mut output = Output::default();
        output.address= Some(destination.clone());
        output.data = data;

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

        let output = Output::new(&address, self.balance());
        self.transaction.outputs.push(output);
        self
    }

}