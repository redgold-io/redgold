use std::collections::HashSet;
use async_trait::async_trait;
use metrics::{counter, gauge};
use redgold_schema::parties::{PartyInstance, PartyMetadata, PartyState};
use redgold_schema::RgResult;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use redgold_common::external_resources::{ExternalNetworkResources, PartyCreationResult, PeerBroadcast};
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::eth::safe_multisig::SafeMultisig;
use redgold_keys::monero::node_wrapper::{MoneroNodeRpcInterfaceWrapper, PartySecretData, PartySecretInstanceData, StateHistoryItem};
use redgold_keys::solana::derive_solana::ToSolanaAddress;
use redgold_keys::solana::wallet::SolanaNetwork;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, PublicKey, Request, Response, SupportedCurrency, Weighting};
use redgold_schema::util::times::current_time_millis;




#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub struct PartyUpdateEvents {
    pub new_instances: Vec<PartyInstance>,
    pub updated_metadata: Option<PartyMetadata>,
    pub new_secrets: Vec<PartySecretInstanceData>,
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

pub async fn check_formations<E: ExternalNetworkResources, B: PeerBroadcast>(
    metadata: &PartyMetadata,
    ext: &E,
    self_hot_addresses: &Vec<Address>,
    party_peer_keys: &Vec<PublicKey>,
    peer_broadcast: &B,
    self_public_key: &PublicKey,
    override_threshold: Option<Weighting>,
    self_private_key_hex: &String,
    network: &NetworkEnvironment,
    words_pass: WordsPass
)
    -> RgResult<PartyUpdateEvents> {

    counter!("redgold_party_formation_tick").increment(1);
    let mut event = PartyUpdateEvents::default();
    let currencies = SupportedCurrency::multisig_party_currencies();

    let current_currencies = metadata.instances_proposed_by(self_public_key).iter()
        .map(|a| a.currency())
        .collect::<HashSet<SupportedCurrency>>();
    gauge!("redgold_party_formation_current_currencies").set(current_currencies.len() as f64);


    let missing = currencies.iter()
        .filter(|c| !current_currencies.contains(c))
        .collect::<Vec<&SupportedCurrency>>();

    gauge!("redgold_party_formation_missing_currencies").set(missing.len() as f64);

    if missing.is_empty() {
        return Ok(event);
    }

    let mut all_pks = vec![self_public_key.clone()];
    all_pks.extend(party_peer_keys.clone());

    all_pks.sort_by(|a, b| a.vec().cmp(&b.vec()));

    let override_thresh_int = override_threshold.map(|w| {
        if let Some(b) = w.basis {
            let fract = (w.value as f64) / (b as f64);
            let num_peers = fract * (all_pks.len() as f64);
            num_peers as i64
        } else {
            w.value
        }
    });

    let mut threshold = override_thresh_int.unwrap_or((all_pks.len() / 2) as i64);
    if threshold < 1 {
        threshold = 1;
    }
    if all_pks.len() == 3 {
        threshold = 2;
    }


    for cur in currencies {
        if !metadata.has_instance(cur) {
            if let Ok(Some((instance, creation_result))) = attempt_form_for_currency(
                ext, self_hot_addresses, self_public_key, self_private_key_hex,
                network, &words_pass, &mut all_pks, threshold, &cur,
                peer_broadcast
            ).await.log_error() {
                event.new_instances.push(instance);
                if let Some(j) = creation_result.secret_json {
                    let result = j.json_from::<PartySecretInstanceData>()?;
                    event.new_secrets.push(result);
                }
            }
        }
    }
    if event.new_instances.len() > 0 {
        let mut updated_metadata = metadata.clone();
        event.new_instances.iter()
        .for_each(|i| updated_metadata.add_instance_equal_members(i, &all_pks));
        event.updated_metadata = Some(updated_metadata);
    }
    Ok(event)
}

async fn attempt_form_for_currency<E, B>(
    ext: &E,
    self_hot_addresses: &Vec<Address>,
    self_public_key: &PublicKey,
    self_private_key_hex: &String,
    network: &NetworkEnvironment,
    words_pass: &WordsPass,
    mut all_pks: &mut Vec<PublicKey>,
    threshold: i64,
    cur: &SupportedCurrency,
    peer_broadcast: &B,
) -> RgResult<Option<(PartyInstance, PartyCreationResult)>> where E: ExternalNetworkResources, B: PeerBroadcast {
        // New party instance required:
        let fee = estimated_fee_min_balance_contract_multisig_establish(cur.clone());
        let address = self_hot_addresses.iter().find(|a| &a.currency() == cur);
        let mut can_form = !SupportedCurrency::multisig_contract_fees().contains(&cur);
        if let (Some(f), Some(a)) = (fee, address) {
            if let Ok(b) = ext.get_live_balance(a).await.log_error() {
                if b >= f {
                    // Create new party instance
                    can_form = true;
                } else {
                    error!("Insufficient balance {} for party formation fee: {} {}", b.to_fractional(), f.to_fractional(), a.render_string().unwrap());
                }
            }
        }
        if can_form {
            let mut new_instance = PartyInstance::default();
            new_instance.threshold = Some(Weighting::from_int_basis(threshold, all_pks.len() as i64));
            new_instance.proposer = Some(self_public_key.clone());
            new_instance.state = PartyState::Active as i32;
            new_instance.creation_time = Some(current_time_millis());
            new_instance.last_update_time = Some(current_time_millis());
            let creation_result = ext.create_multisig_party(
                cur, &all_pks, self_public_key, self_private_key_hex, network, words_pass.clone(), threshold, peer_broadcast, &all_pks
            ).await?;
            if let Some(c) = creation_result {
                new_instance.address = Some(c.address.clone());
                Ok(Some((new_instance, c)))
            } else {
                Ok(None)
            }
        } else {
            "Insufficient balance for party formation".to_error()
        }
}
