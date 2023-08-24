use serde::{Deserialize, Serialize};
use redgold_schema::servers::Server;
use redgold_schema::structs::{Address, PeerId, TrustRatingLabel};

#[derive(Serialize, Deserialize, Clone)]
struct NamedXpub {
    name: String,
    xpub: String,
    hardware_id: String
}

#[derive(Serialize, Deserialize, Clone)]
struct ContactAddress {
    name: String,
    address: Address,
    peer_id: Option<PeerId>,
}

#[derive(Serialize, Deserialize, Clone)]
struct WatchedAddress {
    name: String,
    address: Address,
    alert_all: bool,
    alert_outgoing: bool
}


#[derive(Serialize, Deserialize, Clone)]
struct ServerTrustRatingLabels {
    peer_id_index: i64,
    labels: Vec<TrustRatingLabel>
}

#[derive(Serialize, Deserialize, Clone)]
struct LocalStoredState {
    servers: Vec<Server>,
    xpubs: Vec<NamedXpub>,
    trust: Vec<ServerTrustRatingLabels>,
    contacts: Vec<ContactAddress>,
    watched_address: Vec<Address>,
    email_alert_config: Option<String>
}