use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::structs::{PartySigningValidation, SupportedCurrency};
use crate::core::relay::Relay;
use crate::party::party_stream::PartyEvents;

impl PartyEvents {

    pub fn validate_event(&self, validator: PartySigningValidation, hash_to_sign: Vec<u8>, r: &Relay) -> RgResult<()> {
        let c = validator.currency();
        if c == SupportedCurrency::Redgold {
            let tx = validator.transaction.safe_get_msg("Missing transaction")?;
            return self.validate_rdg_swap_fulfillment_transaction(tx);
        }
        let payload = validator.json_payload.safe_get_msg("Missing PSBT")?.clone();
        match c {
            SupportedCurrency::Bitcoin => {
                let mut w = r.btc_wallet(&self.party_public_key)?;
                self.validate_btc_fulfillment(payload, hash_to_sign, &mut w)?;
            }
            SupportedCurrency::Ethereum => {
                self.validate_eth_fulfillment(payload, hash_to_sign)?;
            }
            _ => {
                return Err(error_info("Unsupported currency"));
            }
        }
        Ok(())
    }
}