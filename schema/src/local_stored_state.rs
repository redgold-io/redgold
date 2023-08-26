use serde::{Deserialize, Serialize};
use crate::servers::Server;
use crate::structs::{Address, PeerId, TrustRatingLabel};

#[derive(Serialize, Deserialize, Clone)]
pub struct NamedXpub {
    pub name: String,
    pub xpub: String,
    pub hardware_id: Option<String>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContactAddress {
    name: String,
    address: Address,
    peer_id: Option<PeerId>,
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
    peer_id_index: i64,
    labels: Vec<TrustRatingLabel>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalStoredState {
    pub servers: Vec<Server>,
    pub xpubs: Vec<NamedXpub>,
    pub trust: Vec<ServerTrustRatingLabels>,
    pub contacts: Vec<ContactAddress>,
    pub watched_address: Vec<Address>,
    pub email_alert_config: Option<String>
}

impl Default for LocalStoredState {
    fn default() -> Self {
        Self {
            servers: vec![],
            xpubs: vec![],
            trust: vec![],
            contacts: vec![],
            watched_address: vec![],
            email_alert_config: None
        }
    }
}