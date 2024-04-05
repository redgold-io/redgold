use bdk::bitcoin::secp256k1::{PublicKey, SecretKey};
use itertools::Itertools;
use log::info;
use redgold_data::data_store::DataStore;
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY};
use redgold_schema::{bytes_data, EasyJson, error_info, RgResult, SafeOption, structs, WithMetadataHashable};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{Address, AddressInfo, CodeExecutionContract, CurrencyAmount, ErrorInfo, ExecutorBackend, Input, LiquidityDeposit, LiquidityRange, LiquidityRequest, NetworkEnvironment, NodeMetadata, Observation, Output, OutputContract, OutputType, PeerMetadata, PoWProof, StandardContractType, StandardData, SupportedCurrency, Transaction, TransactionData, TransactionOptions, UtxoEntry};
use redgold_schema::transaction::amount_data;
use crate::api::public_api::PublicClient;
use redgold_schema::fee_validator::{MIN_RDG_SATS_FEE, TransactionFeeValidator};
use redgold_schema::tx_schema_validate::SchemaValidationSupport;
use crate::node_config::NodeConfig;

// Really just move the transaction builder to the main thing??

//
// pub trait TransactionBuilderSupportAll {
//
// }
//
// impl TransactionBuilderSupportAll for TransactionBuilder {
//     fn with_ds(&self, ds: DataStore)
// }


pub trait TransactionBuilderSupport {
    fn new(network: &NodeConfig) -> Self;
}

impl TransactionBuilderSupport for TransactionBuilder {
    fn new(config: &NodeConfig) -> Self {
        let tx = Transaction::new_blank();
        let network = config.network.clone();
        let fee_addrs = config.seed_peer_addresses();
        let mut s = Self {
            transaction: tx,
            utxos: vec![],
            used_utxos: vec![],
            ds: None,
            client: None,
            network: Some(network.clone()),
            nc: Some(config.clone()),
            fee_addrs,
            allow_bypass_fee: false,
        };
        s.with_network(&network);
        s
    }
}

pub struct TransactionBuilder {
    pub transaction: Transaction,
    pub utxos: Vec<UtxoEntry>,
    pub used_utxos: Vec<UtxoEntry>,
    // TODO: These can be injected as traits to get utxos.
    pub ds: Option<DataStore>,
    pub client: Option<PublicClient>,
    pub network: Option<NetworkEnvironment>,
    pub nc: Option<NodeConfig>,
    pub fee_addrs: Vec<Address>,
    pub allow_bypass_fee: bool
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
        opts.transaction_type = Some(transaction_type as i32);
        self
    }
    pub fn with_ds(&mut self, ds: DataStore) -> &mut Self {
        self.ds = Some(ds);
        self
    }

    pub fn with_client(&mut self, client: PublicClient) -> &mut Self {
        self.client = Some(client);
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
        let first_fee_addr = self.fee_addrs.get(0).safe_get_msg("Missing fee address")?.clone().clone();
        self.with_fee(&first_fee_addr, &CurrencyAmount::min_fee())?;
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
    pub fn with_last_output_deposit_swap_fulfillment(&mut self, btc_txid: String) -> &mut Self {
        if let Some(o) = self.transaction.outputs.last_mut() {
            if let Some(d) = o.data.as_mut() {
                d.external_transaction_id = Some(structs::ExternalTransactionId {identifier: btc_txid.clone()});
            }
        }
        self.with_last_output_swap_type();
        self
    }

    pub fn with_last_output_type(&mut self, output_type: OutputType) -> &mut Self {
        if let Some(o) = self.transaction.outputs.last_mut() {
            o.output_type = Some(output_type as i32);
        };
        self
    }

    pub fn with_last_output_swap_type(&mut self) -> &mut Self {
        let contract_type = structs::StandardContractType::Swap;
        self.with_last_output_contract_type(contract_type);
        self
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


    pub fn with_stake_usd_bounds(&mut self, lower: Option<f64>, upper: Option<f64>, address: &Address) -> &mut Self {
        let mut o = Output::default();
        o.address = Some(address.clone());
        let mut d = StandardData::default();
        let mut lq = LiquidityRequest::default();
        let mut deposit = LiquidityDeposit::default();
        let mut lr = LiquidityRange::default();
        lr.min_inclusive = lower.map(|lower| CurrencyAmount::from_fractional(lower).expect("works"));
        lr.max_exclusive = upper.map(|upper| CurrencyAmount::from_fractional(upper).expect("works"));
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

    pub fn balance(&self) -> i64 {
        self.transaction.total_input_amount() - self.transaction.total_output_amount()
    }

    pub fn build(&mut self) -> Result<Transaction, ErrorInfo> {

        // TODO: Should we sort the other way instead? To compress as many UTXOs as possible?
        // In order for that we'd need to raise transaction size limit
        self.utxos.sort_by(|a, b| a.amount().cmp(&b.amount()));

        for u in self.utxos.clone() {
            if self.balance() > 0 {
                break
            }
            self.with_unsigned_input(u.clone())?;
        }

        if self.balance() < 0 {
            return Err(ErrorInfo::error_info("Insufficient funds"));
        }

        if self.balance() > 0 {
            self.with_remainder();
        }

        if !self.transaction.validate_fee(&self.fee_addrs) {
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
            if !self.transaction.validate_fee(&self.fee_addrs) && !self.allow_bypass_fee {
                return Err(ErrorInfo::error_info("Insufficient fee")).add(self.transaction.json_or())
                    .add("Valid Fee Addresses:")
                    .add(self.fee_addrs.iter().map(|a| a.render_string().expect("works")).join(", "));
            };
        }

        self.with_pow()?;

        self.transaction.validate_schema(self.network.as_ref(), false)?;
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
