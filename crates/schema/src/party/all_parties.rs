use crate::structs::{PublicKey};
use crate::util::times;
use itertools::Itertools;
use std::collections::HashMap;
use crate::parties::{PartyState, PartyInfo};

#[derive(Clone)]
pub struct AllParties {
    parties: Vec<PartyInfo>,
    // by owner
    pub grouped: HashMap<PublicKey, Vec<PartyInfo>>,
    pub active: Vec<PartyInfo>,
}

impl AllParties {
    pub fn new(parties: Vec<PartyInfo>) -> Self {
        let mut grouped = HashMap::new();
        for p in &parties {
            if !p.not_debug() {
                continue
            }
            if let Some(host_key) = p.host_public_key() {
                grouped.entry(host_key.clone()).or_insert_with(Vec::new).push(p.clone());
            }
        }
        let mut active_parties = vec![];
        for (_, parties) in grouped.iter_mut() {
            parties.sort_by_key(|p| p.time().expect("time missing"));
            let priors = parties.iter()
                .flat_map(|p| p.prior_keys().clone())
                .flat_map(|x| x.clone())
                .unique()
                .collect_vec();

            for party in parties {
                let mut active_party = false;
                if let Some(party_key) = party.host_public_key() {
                    let deprecated_party = priors.contains(party_key);
                    let has_successor = party.successor_key.is_some();
                    let month_ago = times::current_time_millis() - 1000 * 60 * 60 * 24 * 30;
                    let expired_time = party.expired_time.unwrap_or(0);
                    let expired = expired_time < month_ago;
                    let marked_as_done = deprecated_party || has_successor;
                    let inactive_party = marked_as_done && expired;
                    active_party = !inactive_party;
                }
                if active_party {
                    active_parties.push(party.clone());
                }
            }
        }

        Self {
            parties,
            grouped,
            active: active_parties,
        }
    }
}