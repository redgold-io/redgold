use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use crate::servers::ServerOldFormat;
use crate::structs::{Address, PeerId, PublicKey, TrustRatingLabel};



#[derive(Clone, Debug, EnumIter, EnumString, PartialEq, Serialize, Deserialize)]
pub enum XPubRequestType {
    Cold,
    Hot,
    QR
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NamedXpub {
    pub name: String,
    pub derivation_path: String,
    pub xpub: String,
    pub hot_offset: Option<String>,
    pub key_name_source: Option<String>,
    pub device_id: Option<String>,
    pub key_reference_source: Option<String>,
    // Maybe should be same as 'name' but right now name references a hot key
    pub key_nickname_source: Option<String>,
    pub request_type: Option<XPubRequestType>,
    pub skip_persist: Option<bool>
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SavedAddress {
    name: String,
    address: Address,
    contact_name: String
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WatchedAddress {
    name: String,
    address: Address,
    alert_all: bool,
    alert_outgoing: bool
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ServerTrustRatingLabels {
    pub peer_id_index: i64,
    pub labels: Vec<TrustRatingLabel>,
    pub environment: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Identity {
    pub name: String,
    pub peer_id_index: i64,
    pub xpub_name: String
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Contact {
    pub name: String,
    pub peer_id: Option<PeerId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct StoredMnemonic {
    pub name: String,
    pub mnemonic: String,
    // pub passphrase: Option<String>,
    pub persist_disk: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct StoredPrivateKey {
    pub name: String,
    pub key_hex: String,
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalStoredState {
    pub servers: Vec<ServerOldFormat>,
    pub xpubs: Vec<NamedXpub>,
    pub trust: Vec<ServerTrustRatingLabels>,
    pub saved_addresses: Option<Vec<SavedAddress>>,
    pub contacts: Vec<Contact>,
    pub watched_address: Vec<Address>,
    pub email_alert_config: Option<String>,
    pub identities: Vec<Identity>,
    pub mnemonics: Option<Vec<StoredMnemonic>>,
    pub private_keys: Option<Vec<StoredPrivateKey>>,
}

impl LocalStoredState {
    pub fn clear_sensitive(&mut self) {
        self.mnemonics = self.mnemonics.clone().map(|mnemonics| {
            mnemonics.iter().filter(|mnemonic| {
                mnemonic.persist_disk.unwrap_or(true)
            }).map(|d| d.clone()).collect_vec()
        });
        self.xpubs = self.xpubs.iter().filter(|xpubs| {
            !xpubs.skip_persist.unwrap_or(false)
        }).map(|d| d.clone()).collect_vec();

    }
}

impl LocalStoredState {
    pub fn key_names(&self) -> Vec<String> {
        let mut k = vec!["default".to_string()];
        for key in self.mnemonics.as_ref().unwrap_or(&vec![]) {
            k.push(key.name.clone());
        }
        for key in self.private_keys.as_ref().unwrap_or(&vec![]) {
            k.push(key.name.clone());
        }
        k
    }
    pub fn by_key(&self, name: &String) -> Option<Either<StoredMnemonic, StoredPrivateKey>> {
        if let Some(mnemonics) = &self.mnemonics {
            for mnemonic in mnemonics {
                if &mnemonic.name == name {
                    return Some(Either::Left(mnemonic.clone()));
                }
            }
        }
        if let Some(private_keys) = &self.private_keys {
            for private_key in private_keys {
                if &private_key.name == name {
                    return Some(Either::Right(private_key.clone()));
                }
            }
        }
        None
    }
}