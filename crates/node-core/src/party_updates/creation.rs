use redgold_schema::parties::PartyMetadata;
use redgold_schema::RgResult;
use serde::{Deserialize, Serialize};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::{Address, CurrencyAmount, SupportedCurrency};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
struct PartyUpdateEvents {

}

pub fn estimated_fee_min_balance_contract_multisig_establish(
    currency: SupportedCurrency
) -> Option<CurrencyAmount> {
    match currency {
        SupportedCurrency::Solana => {
            // Squads v3 multisig creation fee
            // Approximately 0.05 SOL for creation
            // Reference: https://squads.so/pricing
            Some(CurrencyAmount::from_fractional_cur(0.05, SupportedCurrency::Solana).unwrap())
        }
        SupportedCurrency::Ethereum => {
            // Gnosis Safe creation gas costs
            // Approximately 250k-300k gas
            // At 50 gwei gas price = ~0.015 ETH
            // Reference: https://help.safe.global/en/articles/4934378-what-are-the-costs-for-creating-a-safe
            Some(CurrencyAmount::from_fractional_cur(0.015, SupportedCurrency::Ethereum).unwrap())
        }
        _ => None
    }
}

pub async fn check_formations<E: ExternalNetworkResources>(
    metadata: PartyMetadata,
    ext: E,
    self_hot_addresses: Vec<Address>
)
    -> RgResult<PartyUpdateEvents> {

    let events = PartyUpdateEvents::default();

    for cur in SupportedCurrency::multisig_party_currencies() {
        if !metadata.has_instance(cur) {
            // New party instance required:
            let fee = estimated_fee_min_balance_contract_multisig_establish(cur);
            let address = self_hot_addresses.iter().find(|a| a.currency() == cur);
            if let (Some(f), Some(a)) = (fee, address) {
                if let Ok(b) = ext.get_live_balance(a).await.log_error() {

                }
            }
        }
    }

    Ok(events)
}
