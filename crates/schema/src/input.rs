use crate::SafeOption;
use crate::structs::{Address, AddressSelector, ErrorInfo, FloatingUtxoId, Input, Proof};

impl Input {

    pub fn address(&self) -> Result<Address, ErrorInfo> {
        let result = self.output.as_ref().ok_msg("Missing output")?;
        result.address.clone().ok_msg("Missing address")
    }

    pub fn predicate_filter(address: &Address) -> Self {
        let mut input = Self::default();
        let mut f = FloatingUtxoId::default();
        let mut a = AddressSelector::default();
        a.address = Some(address.clone());
        a.requires_output_contract_predicate_match = Some(true);
        f.address_selector = Some(a);

        input.floating_utxo_id = Some(f);
        input
    }

}