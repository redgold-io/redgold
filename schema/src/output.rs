
use crate::structs::{ContentionKey, ErrorInfo, Hash, Output, OutputType, StandardContractType, StateSelector, CurrencyAmount, UtxoEntry, Observation, StandardData, StandardRequest, StandardResponse, SwapFulfillment};
use crate::transaction::amount_data;
use crate::{Address, HashClear, RgResult, SafeOption};

pub fn output_data(address: Vec<u8>, amount: u64) -> Output {
    Output::new(&Address::address_data(address).expect(""), amount as i64)
}

pub fn tx_output_data(address: Address, amount: u64) -> Output {
    Output::new(&address, amount as i64)
}

impl HashClear for Output {
    fn hash_clear(&mut self) {}
}

impl Output {

    pub fn is_request(&self) -> bool {
        self.output_type == Some(OutputType::RequestCall as i32)
    }

    pub fn is_deploy(&self) -> bool {
        self.output_type == Some(OutputType::Deploy as i32)
    }

    pub fn code(&self) -> Option<Vec<u8>> {
        self.contract.as_ref()
            .and_then(|d| d.code_execution_contract.as_ref())
            .and_then(|d| d.code.as_ref())
            .map(|d| d.value.clone())
    }

    pub fn validate_deploy_code(&self) -> RgResult<Vec<u8>> {
        // Validate deploy
        if self.is_deploy() {
            if let Some(d) = self.contract.as_ref()
                .and_then(|d| d.code_execution_contract.as_ref())
                .and_then(|d| d.code.as_ref())
                .map(|d| d.value.clone())
                .filter(|d| !d.is_empty())
                .filter(|d| Address::script_hash(d).ok() == self.address)
            {
                return Ok(d);
            }
        }
        Err(ErrorInfo::error_info("Not a deploy"))
    }

    pub fn pay_update_descendents(&self) -> bool {
        self.contract.as_ref().map(|c| c.pay_update_descendents).unwrap_or(false)
    }

    pub fn request_data(&self) -> RgResult<&Vec<u8>> {
        if self.is_request() {
            if let Some(d) = self.data.as_ref().and_then(|d| d.request.as_ref()) {
                return Ok(&d.value);
            }
        }
        Err(ErrorInfo::error_info("Not a request"))
    }

    pub fn request_contention_key(&self) -> RgResult<ContentionKey> {
        let option = self.address.safe_get_msg("Missing address")?;
        let sel = self.request_selector()?;
        Ok(ContentionKey::contract_request(option, sel))
    }

    pub fn request_selector(&self) -> RgResult<Option<&StateSelector>> {
        if self.is_request() {
            return Ok(self.data.as_ref().and_then(|d|
                d.standard_request.as_ref().and_then(|r| r.selector.as_ref())));
        }
        Err(ErrorInfo::error_info("Not a request"))
    }

    pub fn new(address: &Address, amount: i64) -> Output {
        Output {
            address: Some(address.clone()),
            product_id: None,
            counter_party_proofs: vec![],
            data: amount_data(amount as u64),
            contract: None,
            output_type: None,
            utxo_id: None
        }
    }
    pub fn from_data(data: StandardData) -> Self {
        let mut o = Output::default();
        o.data = Some(data);
        o
    }

    pub fn is_swap(&self) -> bool {
        self.contract.as_ref().and_then(|c| c.standard_contract_type)
            .filter(|&c| c == StandardContractType::Swap as i32).is_some()
    }

    pub fn is_peer_data(&self) -> bool {
        self.data.as_ref().and_then(|c| c.peer_data.as_ref()).is_some()
    }

    pub fn is_node_metadata(&self) -> bool {
        self.data.as_ref().and_then(|c| c.node_metadata.as_ref()).is_some()
    }

    pub fn is_metadata(&self) -> bool {
        self.is_node_metadata() || self.is_peer_data()
    }

    pub fn request(&self) -> Option<&StandardRequest> {
        self.data.as_ref().and_then(|d| d.standard_request.as_ref())
    }

    pub fn response(&self) -> Option<&StandardResponse> {
        self.data.as_ref().and_then(|d| d.standard_response.as_ref())
    }

    pub fn swap_fulfillment(&self) -> Option<&SwapFulfillment> {
        self.response().and_then(|r| r.swap_fulfillment.as_ref())
    }

    pub fn is_liquidity(&self) -> bool {
        self.request().and_then(|c| c.liquidity_request.as_ref()).is_some()
    }

    pub fn is_fee(&self) -> bool {
        self.output_type == Some(OutputType::Fee as i32)
    }

    pub fn utxo_entry(
        &self,
        transaction_hash: &Hash,
        output_index: i64,
        time: i64,
    ) -> UtxoEntry {
        return UtxoEntry::from_output_new(
            self, &transaction_hash,
            output_index, time
        );
    }

    pub fn amount(&self) -> u64 {
        self.amount_i64() as u64
    }

    pub fn amount_i64(&self) -> i64 {
        self.data.as_ref().unwrap().amount.clone().unwrap().amount
    }

    pub fn safe_ensure_amount(&self) -> Result<&i64, ErrorInfo> {
        Ok(&self.data.safe_get_msg("Missing data field on output")?
            .amount.safe_get_msg("Missing amount field on output")?.amount)
    }

    pub fn observation(&self) -> RgResult<&Observation> {
        self.data.safe_get_msg("Missing data field on output")?
            .observation.safe_get_msg("Missing observation field on output")
    }

    pub fn opt_amount(&self) -> Option<i64> {
        self.data.safe_get_msg("Missing data field on output").ok()
            .and_then(|data| data.amount.as_ref())
            .map(|data| data.amount)
    }

    pub fn opt_amount_typed(&self) -> Option<CurrencyAmount> {
        self.data.safe_get_msg("Missing data field on output").ok().and_then(|data| data.amount.clone())
    }

    pub fn opt_amount_typed_ref(&self) -> Option<&CurrencyAmount> {
        self.data.as_ref().and_then(|data| data.amount.as_ref())
    }

    pub fn rounded_amount(&self) -> f64 {
        crate::transaction::rounded_balance(self.amount())
    }
}
