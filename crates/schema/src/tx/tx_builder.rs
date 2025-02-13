use std::collections::HashMap;
use crate::conf::node_config::NodeConfig;
use crate::fee_validator::{TransactionFeeValidator, MIN_RDG_SATS_FEE};
use crate::helpers::easy_json::EasyJson;
use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::observability::errors::EnhanceErrorInfo;
use crate::structs::{Address, AddressDescriptor, AddressInfo, CodeExecutionContract, CurrencyAmount, DepositRequest, ErrorInfo, ExecutorBackend, ExternalTransactionId, Input, LiquidityRange, NetworkEnvironment, NodeMetadata, Observation, Output, OutputContract, OutputType, PeerMetadata, PoWProof, StakeDeposit, StakeRequest, StakeWithdrawal, StandardContractType, StandardData, StandardRequest, StandardResponse, SupportedCurrency, Transaction, TransactionData, TransactionOptions, UtxoEntry, UtxoId};
use crate::transaction::amount_data;
use crate::tx_schema_validate::SchemaValidationSupport;
use crate::{bytes_data, error_info, structs, RgResult, SafeOption};
use itertools::Itertools;
use log::info;

#[derive(Clone)]
pub struct TransactionBuilder {
    pub transaction: Transaction,
    pub utxos: Vec<UtxoEntry>,
    pub used_utxos: Vec<UtxoEntry>,
    pub used_utxo_ids: Vec<UtxoId>,
    pub network: Option<NetworkEnvironment>,
    pub nc: Option<NodeConfig>,
    pub fee_addrs: Vec<Address>,
    pub allow_bypass_fee: bool,
    pub input_addresses: Vec<Address>,
    pub input_addresses_descriptors: Vec<AddressDescriptor>,
    pub zero_fee_requested: bool
}


impl TransactionBuilder {
    pub fn with_input_address(&mut self, p0: &Address) -> &mut TransactionBuilder {
        self.input_addresses.push(p0.clone());
        self
    }
    pub fn with_input_address_descriptor(&mut self, p0: &AddressDescriptor) -> &mut TransactionBuilder {
        self.input_addresses_descriptors.push(p0.clone());
        self
    }

}

impl TransactionBuilder {

    pub fn with_pow(&mut self) -> RgResult<&mut Self> {
        let hash = self.transaction.signable_hash();
        let proof = PoWProof::from_hash_sha3_256(&hash, 1)?;
        let opts = self.transaction.options.as_mut().expect("");
        opts.pow_proof = Some(proof);
        Ok(self)
    }
    pub fn with_type(&mut self, transaction_type: structs::TransactionType) -> &mut Self {
        let opts = self.transaction.options.as_mut().expect("");
        opts.transaction_type = transaction_type as i32;
        self
    }

    pub fn with_network(&mut self, network: &NetworkEnvironment) -> &mut Self {
        self.network = Some(network.clone());
        let mut options = self.transaction.options.clone().unwrap_or(TransactionOptions::default());
        options.network_type = Some(network.clone() as i32);
        self.transaction.options = Some(options);
        self
    }

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
    pub fn with_is_test(&mut self) -> &mut Self{
        let mut options = self.transaction.options.clone().unwrap_or(TransactionOptions::default());
        options.is_test = Some(true);
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
        self.with_type(structs::TransactionType::ObservationType);
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
        let o = option.ok_or(error_info("Missing output"))?;
        o.output_type = Some(OutputType::Fee as i32);
        Ok(self)
    }

    pub fn with_default_fee(&mut self) -> RgResult<&mut Self> {
        let first_fee_addr = (*self.fee_addrs.get(0).safe_get_msg("Missing fee address")?).clone();
        self.with_fee(&first_fee_addr, &CurrencyAmount::min_fee())?;
        Ok(self)
    }

    pub fn with_zero_fee_requested(&mut self) -> &mut Self {
        self.zero_fee_requested = true;
        self
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

    pub fn with_nmd_utxo(&mut self, utxo_entry: &UtxoEntry) -> Result<&mut Self, ErrorInfo> {
        let o = utxo_entry.output.safe_get_msg("Missing output")?;
        o.address.safe_get_msg("Missing address")?;
        if !o.is_node_metadata() {
            return Err(ErrorInfo::error_info("Not a node metadata output"));
        }
        self.with_unsigned_input(utxo_entry.clone())?;
        Ok(self)
    }


    pub fn with_utxos(&mut self, utxo_entry: &Vec<UtxoEntry>) -> Result<&mut Self, ErrorInfo> {
        for x in utxo_entry {
            self.with_maybe_currency_utxo(x)?;
        }
        Ok(self)
    }

    pub fn with_maybe_currency_utxo(&mut self, utxo_entry: &UtxoEntry) -> Result<&mut Self, ErrorInfo> {
        let o = utxo_entry.output.safe_get_msg("Missing output")?;
        if let Ok(_a) = o.safe_ensure_amount() {
            self.with_utxo(utxo_entry)?;
        }
        Ok(self)
    }

    pub fn with_message(&mut self, msg: impl Into<String>) -> Result<&mut Self, ErrorInfo> {
        let x = self.transaction.options.as_mut().expect("");
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
            self.with_maybe_currency_utxo(&u)?;
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
    pub fn with_last_output_deposit_swap_fulfillment(&mut self, txid: ExternalTransactionId) -> RgResult<&mut Self> {
        let d = self.last_output_data().ok_or(error_info("Missing output"))?;

        let mut res = StandardResponse::default();
        let mut sw = structs::SwapFulfillment::default();
        sw.external_transaction_id = Some(txid);
        res.swap_fulfillment = Some(sw);
        d.standard_response = Some(res);
        Ok(self)
    }
    pub fn with_last_output_stake_withdrawal_fulfillment(&mut self, initiating_utxo_id: &UtxoId) -> RgResult<&mut Self> {
        let d = self.last_output_data().ok_or(error_info("Missing output"))?;
        let mut res = StandardResponse::default();
        let mut sw = structs::StakeWithdrawalFulfillment::default();
        sw.stake_withdrawal_request = Some(initiating_utxo_id.clone());
        res.stake_withdrawal_fulfillment = Some(sw);
        d.standard_response = Some(res);
        Ok(self)
    }

    pub fn with_last_output_type(&mut self, output_type: OutputType) -> &mut Self {
        if let Some(o) = self.transaction.outputs.last_mut() {
            o.output_type = Some(output_type as i32);
        };
        self
    }

    pub fn last_output(&mut self) -> Option<&mut Output> {
        self.transaction.outputs.last_mut()
    }

    pub fn last_output_data(&mut self) -> Option<&mut StandardData> {
        self.transaction.outputs.last_mut().and_then(|o| o.data.as_mut())
    }

    pub fn last_output_request_or(&mut self) -> Option<&mut StandardRequest> {
        self.last_output_data().and_then(|d| {
            let mut default = StandardRequest::default();
            let r = d.standard_request.as_mut().unwrap_or(&mut default);
            d.standard_request = Some(r.clone());
            d.standard_request.as_mut()
        })
    }

    pub fn with_last_output_swap_type(&mut self) -> &mut Self {
        let contract_type = structs::StandardContractType::Swap;
        self.with_last_output_contract_type(contract_type);
        self
    }

    pub fn with_last_output_request(&mut self, req: StandardRequest) -> &mut Self {
        if let Some(o) = self.transaction.outputs.last_mut() {
            if let Some(d) = o.data.as_mut() {
                d.standard_request = Some(req);
            }
        }
        self
    }

    pub fn with_last_output_swap_destination(&mut self, destination: &Address) -> RgResult<&mut Self> {
        let mut addr = destination.clone();
        addr.mark_external();
        let mut swap_request = structs::SwapRequest::default();
        swap_request.destination = Some(addr);
        self.last_output_request_or().ok_msg("Missing output")?.swap_request = Some(swap_request);
        Ok(self)
    }

    pub fn with_swap(&mut self, destination: &Address, party_fee_or_rdg_amount: &CurrencyAmount, party_address: &Address) -> RgResult<&mut Self> {
        self.with_output(party_address, party_fee_or_rdg_amount);
        self.with_last_output_swap_destination(destination)?;
        self.with_last_output_swap_type();
        Ok(self)
    }

    pub fn with_last_output_stake(&mut self) -> &mut Self {
        self.with_last_output_contract_type(StandardContractType::Stake)
    }
    pub fn with_last_output_contract_type(&mut self, contract_type: StandardContractType) -> &mut Self {
        if let Some(o) = self.transaction.outputs.last_mut() {
            let mut oc = OutputContract::default();
            oc.standard_contract_type = Some(contract_type as i32);
            o.contract = Some(oc);
        }
        self
    }


    pub fn with_external_stake_usd_bounds(
        &mut self,
        lower: Option<f64>,
        upper: Option<f64>,
        stake_control_address: &Address,
        external_address: &Address,
        external_amount: &CurrencyAmount,
        pool_address: &Address,
        pool_fee: &CurrencyAmount,
    ) -> &mut Self {
        self.with_output(pool_address, pool_fee);
        self.with_last_output_stake();
        let mut o = Output::default();
        o.address = Some(stake_control_address.clone());
        let mut d = StandardData::default();
        let mut lq = StakeRequest::default();
        let mut deposit = StakeDeposit::default();
        if lower.is_some() || upper.is_some() {
            let mut lr = LiquidityRange::default();
            lr.min_inclusive = lower.map(|lower| CurrencyAmount::from_usd(lower).expect("works"));
            lr.max_exclusive = upper.map(|upper| CurrencyAmount::from_usd(upper).expect("works"));
            deposit.liquidity_ranges = vec![lr];
        }
        let mut dr = DepositRequest::default();
        let mut external = external_address.clone();
        external.mark_external();
        dr.address = Some(external);
        dr.amount = Some(external_amount.clone());
        deposit.deposit = Some(dr);
        lq.deposit = Some(deposit);

        let mut sr = StandardRequest::default();
        sr.stake_request = Some(lq);
        d.standard_request = Some(sr);
        o.data = Some(d);
        self.transaction.outputs.push(o);
        self
    }

    pub fn with_internal_stake_usd_bounds(
        &mut self,
        lower: Option<f64>,
        upper: Option<f64>,
        stake_control_address: &Address,
        party_address: &Address,
        party_send_amount: &CurrencyAmount,
    ) -> &mut Self {
        self.with_output(party_address, party_send_amount);
        self.with_last_output_stake();
        let mut o = Output::default();
        o.address = Some(stake_control_address.clone());
        let mut d = StandardData::default();
        let mut lq = StakeRequest::default();
        let mut deposit = StakeDeposit::default();
        if lower.is_some() || upper.is_some() {
            let mut lr = LiquidityRange::default();
            lr.min_inclusive = lower.map(|lower| CurrencyAmount::from_usd(lower).expect("works"));
            lr.max_exclusive = upper.map(|upper| CurrencyAmount::from_usd(upper).expect("works"));
            deposit.liquidity_ranges = vec![lr];
        }
        lq.deposit = Some(deposit);
        let mut sr = StandardRequest::default();
        sr.stake_request = Some(lq);
        d.standard_request = Some(sr);
        o.data = Some(d);
        self.transaction.outputs.push(o);
        self
    }

    pub fn with_stake_withdrawal(&mut self,
                                 destination: &Address,
                                 party_address: &Address,
                                 party_fee: &CurrencyAmount,
        original_utxo: &UtxoId
    ) -> &mut Self {
        self.with_direct_input(original_utxo);
        let mut o = Output::default();
        o.address = Some(party_address.clone());
        let mut d = StandardData::default();
        d.amount = Some(party_fee.clone());
        let mut lq = StakeRequest::default();
        let mut withdrawal = StakeWithdrawal::default();
        let mut destination = destination.clone();
        destination.mark_external();
        withdrawal.destination = Some(destination.clone());
        lq.withdrawal = Some(withdrawal);
        let mut sr = StandardRequest::default();
        sr.stake_request = Some(lq);
        d.standard_request = Some(sr);
        o.data = Some(d);
        self.transaction.outputs.push(o);
        self
    }

        // Aha heres the issue, we're expecting output to be populated here
    // Remove this assumption elsewhere
        /// TODO: investigate if this enriched output is causing a problem on the input
    pub fn with_unsigned_input(&mut self, utxo: UtxoEntry) -> Result<&mut Self, ErrorInfo> {
        let input = utxo.to_input();
        let output = utxo.output.safe_get()?;
        let _data = output.data.safe_get()?;
        // if let Some(a) = data.amount {
        //     self.balance += a;
        // }
        self.used_utxos.push(utxo.clone());
        self.transaction.inputs.push(input);
        Ok(self)
    }
    pub fn with_unsigned_input_address_descriptor(&mut self, utxo: UtxoEntry, ad: &AddressDescriptor) -> Result<&mut Self, ErrorInfo> {
        let mut input = utxo.to_input();
        input.address_descriptor = Some(ad.clone());
        // if let Some(a) = data.amount {
        //     self.balance += a;
        // }
        self.used_utxos.push(utxo.clone());
        self.transaction.inputs.push(input);
        Ok(self)
    }

    pub fn with_direct_input(&mut self, utxo_id: &UtxoId) -> &mut Self {
        let mut input = Input::default();
        input.utxo_id = Some(utxo_id.clone());
        self.transaction.inputs.push(input);
        self
    }

    pub fn balance(&self) -> i64 {
        self.transaction.total_input_amount() - self.transaction.total_output_amount()
    }

    pub fn build(&mut self) -> Result<Transaction, ErrorInfo> {

        // TODO: Should we sort the other way instead? To compress as many UTXOs as possible?
        // In order for that we'd need to raise transaction size limit
        self.utxos.sort_by(|a, b| a.amount().cmp(&b.amount()));

        let address_descriptors = self.input_addresses_descriptors.iter()
            .map(|a| (a.to_address(), a.clone())).collect::<HashMap<Address, AddressDescriptor>>();

        if address_descriptors.len() > 0 {
            // info!("Address descriptors");
            // for (k,v) in address_descriptors.iter() {
            //     info!("{}: {}", k.json_or(), v.json_or());
            // }
            for u in &self.utxos {
                if let Ok(a) = u.address() {
                    // info!("UTXO address {}", a.json_or());
                    if !address_descriptors.contains_key(&a) {
                        // info!("Missing address descriptor for {}", a.render_string().expect("works"));
                        // info!("Broke");
                    }
                }
            }
        }

        for u in self.utxos.clone() {

            if self.balance() > 0 {
                break
            }
            if u.opt_amount().is_some() {
                if let Some(v) = u.address().ok().and_then(|a| address_descriptors.get(&a)) {
                    self.with_unsigned_input_address_descriptor(u.clone(), v)?;
                } else {
                    self.with_unsigned_input(u.clone())?;
                }
            }
        }

        if self.balance() < 0 {
            return Err(ErrorInfo::error_info("Insufficient funds"));
        }

        if self.balance() > 0 {
            self.with_remainder();
        }

        if !self.transaction.validate_fee_only(&self.fee_addrs) && !self.zero_fee_requested {
            let mut found_fee = false;
            for o in self.transaction.outputs.iter_mut().rev() {
                if let Some(a) = o.data.as_mut().and_then(|data| data.amount.as_mut()) {
                    if a.currency_or() == SupportedCurrency::Redgold && a.amount > MIN_RDG_SATS_FEE {
                        a.amount -= MIN_RDG_SATS_FEE;
                        found_fee = true;
                        // info!("builder Found fee deduction");
                        break;
                    }
                }
            }
            if found_fee {
                self.with_default_fee()?;
            }
            if !self.transaction.validate_fee_only(&self.fee_addrs) && !self.allow_bypass_fee {
                return Err(ErrorInfo::error_info("Insufficient fee")).add(self.transaction.json_or())
                    .add("Valid Fee Addresses:")
                    .add(self.fee_addrs.iter().map(|a| a.render_string().expect("works")).join(", "));
            };
        }

        self.with_pow()?;

        self.transaction.validate_schema(self.network.as_ref(), false)
            .with_detail("tx", self.transaction.json_or())?;
        let transaction = self.transaction.clone();
        Ok(transaction)
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