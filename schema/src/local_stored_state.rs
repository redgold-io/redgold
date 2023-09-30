use serde::{Deserialize, Serialize};
use crate::servers::Server;
use crate::structs::{Address, PeerId, PublicKey, TrustRatingLabel};

#[derive(Serialize, Deserialize, Clone)]
pub struct NamedXpub {
    pub name: String,
    pub derivation_path: String,
    pub xpub: String,
    pub hot_offset: Option<String>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedAddress {
    name: String,
    address: Address,
    contact_name: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WatchedAddress {
    name: String,
    address: Address,
    alert_all: bool,
    alert_outgoing: bool
}


#[derive(Serialize, Deserialize, Clone)]
pub struct ServerTrustRatingLabels {
    pub peer_id_index: i64,
    pub labels: Vec<TrustRatingLabel>
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Identity {
    pub name: String,
    pub peer_id_index: i64,
    pub xpub_name: String
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Contact {
    pub name: String,
    pub peer_id: Option<PeerId>,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct LocalStoredState {
    pub servers: Vec<Server>,
    pub xpubs: Vec<NamedXpub>,
    pub trust: Vec<ServerTrustRatingLabels>,
    pub saved_addresses: Vec<SavedAddress>,
    pub contacts: Vec<Contact>,
    pub watched_address: Vec<Address>,
    pub email_alert_config: Option<String>,
    pub identities: Vec<Identity>
}

impl Default for LocalStoredState {
    fn default() -> Self {
        Self {
            servers: vec![],
            xpubs: vec![],
            trust: vec![],
            saved_addresses: vec![],
            contacts: vec![],
            watched_address: vec![],
            email_alert_config: None,
            identities: vec![],
        }
    }
}