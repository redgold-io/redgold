use crate::structs::{self, Weighting};
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
    pub fn has_instance(&self, cur: SupportedCurrency) -> bool {
        self.instances.iter()
            .flat_map(|i| i.address.as_ref())
            .any(|a| a.currency() == cur)
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