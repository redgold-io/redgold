use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use crate::structs::{self, Address, Weighting};
use crate::structs::{InitiateMultipartyKeygenRequest, LocalKeyShare, MultipartyIdentifier, PublicKey, SupportedCurrency};
use crate::parties::{PartyInfo, PartyInstance, PartyMembership, PartyMetadata, PartyParticipation, PartyState};
impl PartyInfo {

    pub fn active(&self) -> bool {
        self.state == PartyState::Active as i32
    }

    pub fn clear_sensitive(&mut self) -> &mut Self {
        self.local_key_share = None;
        self
    }

    pub fn not_debug(&self) -> bool {
        self.initiate.as_ref()
            .map(|p| p.purpose())
            .filter(|p| p != &structs::PartyPurpose::DebugPurpose)
            .is_some()
    }

    pub fn identifier(&self) -> Option<&MultipartyIdentifier> {
        self.initiate.as_ref().and_then(|i| i.identifier.as_ref())
    }

    pub fn local_share(&self) -> Option<&String> {
        self.local_key_share.as_ref().and_then(|l| l.local_share.as_ref())
    }
    pub fn host_public_key(&self) -> Option<&PublicKey> {
        self.identifier().and_then(|i| i.party_keys.get(0))
    }

    pub fn time(&self) -> Option<i64> {
        self.initiate.as_ref().map(|i| i.time)
    }

    pub fn prior_keys(&self) -> Option<&Vec<PublicKey>> {
        self.initiate.as_ref().map(|i| i.prior_keys.as_ref())
    }

    pub fn new_from(initiate: InitiateMultipartyKeygenRequest, local_share: String, self_initiated: bool) -> Self {
        Self {
            initiate: Some(initiate.clone()),
            local_key_share: Some(LocalKeyShare{
                local_share: Some(local_share.clone()),
                share_type: 0,
                version: None,
            }),
            party_key: None,
            self_initiated: Some(self_initiated),
            expired_time: None,
            successor_key: None,
            state: PartyState::Active as i32,
        }
    }

}


impl PartyMetadata {


    pub fn members_of(&self, address: &Address) -> Vec<PublicKey> {
        self.memberships.iter()
            .filter(|m| m.participate.iter().any(|p| p.address.as_ref() == Some(address)))
            .map(|m| m.public_key.clone().unwrap())
            .collect()
    }
    pub fn address_by_currency(&self) -> HashMap<SupportedCurrency, Vec<Address>> {
        self.instances.iter()
            .group_by(|a| a.address.as_ref().map(|a| a.currency()))
            .into_iter()
            .filter(|(k, _)| k.is_some())
            .map(|(k, v)| (k.clone().unwrap(), v
                .map(|a| a.address.as_ref().unwrap()).cloned().collect()))
            .collect()
    }

    pub fn earliest_time(&self) -> i64 {
        self.instances.iter().filter_map(|i| i.creation_time).min().unwrap_or(0)
    }

    pub fn group_by_proposer(&self) -> HashMap<PublicKey, PartyMetadata> {
        self.instances.iter()
            .filter_map(|i| i.proposer.as_ref())
            .unique()
            .map(|p| (p.clone(), self.filter_by_proposer(p)))
            .collect()
    }

    pub fn filter_by_proposer(&self, key: &PublicKey) -> PartyMetadata {
        let mut ret = self.clone();
        ret.instances = self.instances_proposed_by(key);
        let address = ret.instances.iter()
            .flat_map(|i| i.address.as_ref())
            .collect::<HashSet<&Address>>();

        ret.memberships = self.memberships.iter()
            .map(|m| {
                let mut updated = m.clone();
                updated.participate = updated.participate
                    .iter()
                    .filter(|p| p.address.as_ref().map(|a| address.contains(a)).unwrap_or(false))
                    .cloned()
                    .collect();
                updated
            })
            .collect_vec();
        ret.memberships.retain(|m| m.participate.len() > 0);
        ret
    }

    pub fn instances_proposed_by(&self, key: &PublicKey) -> Vec<PartyInstance> {
        self.instances.iter()
            .filter(|i| i.proposer.as_ref() == Some(key))
            .cloned()
            .collect()
    }

    pub fn active_proposed_by(&self, key: &PublicKey) -> Vec<PartyInstance> {
        self.instances.iter()
            .filter(|i| i.proposer.as_ref() == Some(key))
            .filter(|x| x.is_active())
            .cloned()
            .collect()
    }

    pub fn active(&self) -> Vec<PartyInstance> {
        self.instances.iter()
            .filter(|i| i.is_active())
            .cloned()
            .collect()
    }

    pub fn has_instance(&self, cur: SupportedCurrency) -> bool {
        self.instances.iter()
            .flat_map(|i| i.address.as_ref())
            .any(|a| a.currency() == cur)
    }

    pub fn instances_of(&self, cur: &SupportedCurrency) -> impl Iterator<Item=&PartyInstance> {
        let cur = cur.clone() as i32;
        self.instances.iter()
            .filter(move |i| i.address.as_ref().map(|a| a.currency == cur).unwrap_or(false))
    }

    pub fn instances_of_address(&self, addr: &Address) -> Option<&PartyInstance> {
        self.instances.iter().find(|i| i.address.as_ref() == Some(addr))
    }

    pub fn latest_instance_by(&self, cur: SupportedCurrency) -> Option<&PartyInstance> {
        self.instances_of(&cur).max_by_key(|i| i.creation_time)
    }

    pub fn add_instance_equal_members(&mut self, instance: &PartyInstance, equal_members: &Vec<PublicKey>) {
        let addr = instance.address.clone();
        self.instances.push(instance.clone());
        let mut missing = vec![];
        let basis = equal_members.len() as i64;
        let participate = PartyParticipation {
            address: addr.clone(), 
            weight: Some(Weighting::from_int_basis(1, basis))
        };
        for m in self.memberships.iter_mut() {
            if let Some(pk) = m.public_key.as_ref() {
                if !equal_members.contains(pk) {
                    missing.push(pk.clone());
                } else {
                    m.participate.push(participate.clone());
                }
            }
        }
        for pk in missing {
            self.memberships.push(PartyMembership {
                public_key: Some(pk),
                participate: vec![participate.clone()],
            });
        }
    }
}

impl PartyInstance {
    pub fn is_active(&self) -> bool {
        self.state() == PartyState::Active
    }

    pub fn currency(&self) -> SupportedCurrency {
        self.address.as_ref().map(|a| a.currency()).unwrap_or(SupportedCurrency::Redgold)
    }
}
