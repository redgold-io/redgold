use crate::structs::{InitiateMultipartyKeygenRequest, LocalKeyShare, MultipartyIdentifier, PartyInfo, PublicKey};

impl PartyInfo {

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
            successor_key: None
        }
    }

}