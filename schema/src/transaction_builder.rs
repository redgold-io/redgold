use crate::{Address, bytes_data, error_info, ErrorInfo, PeerMetadata, RgResult, SafeOption, struct_metadata_new, structs, Transaction, WithMetadataHashable};
use crate::structs::{AddressInfo, CodeExecutionContract, ExecutorBackend, UtxoId, Input, NodeMetadata, Output, OutputContract, OutputType, StandardData, CurrencyAmount, TransactionData, TransactionOptions, UtxoEntry, Proof, Observation, LiquidityRange, LiquidityRequest, LiquidityDeposit};
use crate::transaction::amount_data;

pub struct TransactionBuilder {
    pub transaction: Transaction,
    pub utxos: Vec<UtxoEntry>,
    pub used_utxos: Vec<UtxoEntry>
}


impl TransactionBuilder {
    pub fn with_options_height(&mut self, height: i64) -> &mut Self{
        let mut options = self.transaction.options.clone().unwrap_or(TransactionOptions::default());
        let mut data = options.data.clone().unwrap_or(TransactionData::default());
        let mut sd = data.standard_data.clone().unwrap_or(StandardData::default());
        sd.height = Some(height);
        data.standard_data = Some(sd);
        options.data = Some(data);
        self.transaction.options = Some(options);
        self
    }
    // pub fn with_input_utxo_id(&mut self, input_utxo_id: &UtxoId) -> &mut Self {
    //     let mut input = Input::default();
    //     input.utxo_id = Some(input_utxo_id.clone());
    //     self.transaction.inputs.push(input.clone());
    //     self
    // }

    pub fn with_input(&mut self, input: &Input) -> &mut Self {
        self.transaction.inputs.push(input.clone());
        self
    }
    pub fn with_observation(&mut self, o: &Observation, height: i64, address: &Address) -> &mut Self {
        self.with_options_height(height);
        let sd = StandardData::observation(o.clone());
        let mut output = Output::from_data(sd);
        output.address = Some(address.clone());
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_no_salt(&mut self) -> &mut Self {
        if let Some(x) = self.transaction.options.as_mut() {
            x.salt = None;
        }
        self
    }

    pub fn with_time(&mut self, time: Option<i64>) -> &mut Self {
        if let Some(x) = self.transaction.struct_metadata.as_mut() {
            x.time = time;
        }
        self
    }

    pub fn with_fee(&mut self, destination: &Address, amount: &CurrencyAmount) -> RgResult<&mut Self> {
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

    pub fn with_output(&mut self, destination: &Address, amount: &CurrencyAmount) -> &mut Self {
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
        &mut self, code: impl AsRef<[u8]>, c_amount: CurrencyAmount, use_predicate_input: bool) -> RgResult<&mut Self> {
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
                d.external_transaction_id = Some(crate::structs::ExternalTransactionId{identifier: btc_txid.clone()});
            }
        }
        self
    }
    pub fn with_stake(&mut self, lower: f64, upper: f64, address: &Address) -> &mut Self {
        let mut o = Output::default();
        o.address = Some(address.clone());
        let mut d = StandardData::default();
        let mut lq = LiquidityRequest::default();
        let mut deposit = LiquidityDeposit::default();
        let mut lr = LiquidityRange::default();
        lr.min_inclusive = Some(CurrencyAmount::from_fractional(lower).expect("works"));
        lr.max_exclusive = Some(CurrencyAmount::from_fractional(upper).expect("works"));
        deposit.liquidity_ranges = vec![lr];
        lq.deposit = Some(deposit);
        d.liquidity_request = Some(lq);
        o.data = Some(d);
        self.transaction.outputs.push(o);
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

    pub fn with_output_peer_data(&mut self, destination: &Address, pd: PeerMetadata, height: i64) -> &mut Self {
        let mut option = StandardData::peer_data(pd).expect("o");
        option.height = Some(height);
        self.with_options_height(height);
        let mut output = Output::default();
        output.address = Some(destination.clone());
        output.data = Some(option);
        self.transaction.outputs.push(output);
        self
    }

    // Actually can we just leave this empty?
    pub fn genesis_peer_data(&mut self) -> &mut Self {
        let o = self.transaction.outputs.last().expect("o");
        // let pd = o.data.expect("d").peer_data.expect("").peer_id.expect("").peer_id.expect("pk");
        o.utxo_entry(
            &self.transaction.hash_or(), 0,
            self.transaction.time().expect("").clone()
        );
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

    pub fn with_genesis_input(&mut self, address: &Address) -> &mut Self {
        let mut input = Input::default();
        input.input_type = Some(structs::InputType::GenesisInput as i32);
        let mut o = Output::default();
        o.address = Some(address.clone());
        input.output = Some(o);
        self.transaction.inputs.push(input);
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