use crate::conf::server_config::Deployment;
use crate::servers::ServerOldFormat;
use crate::structs::{Address, CurrencyAmount, PublicKey, SupportedCurrency, TrustRatingLabel};
use crate::tx::external_tx::ExternalTimedTransaction;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};


#[derive(Clone, Debug, EnumIter, EnumString, PartialEq, Serialize, Deserialize, Eq)]
pub enum XPubLikeRequestType {
    Cold,
    Hot,
    QR,
    File
}

impl Default for XPubLikeRequestType {
    fn default() -> Self {
        XPubLikeRequestType::Hot
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct AccountKeySource {
    pub name: String,
    pub derivation_path: String,
    pub xpub: String,
    pub hot_offset: Option<String>,
    pub key_name_source: Option<String>,
    pub device_id: Option<String>,
    pub key_reference_source: Option<String>,
    // Maybe should be same as 'name' but right now name references a hot key
    pub key_nickname_source: Option<String>,
    pub request_type: Option<XPubLikeRequestType>,
    pub skip_persist: Option<bool>,
    pub preferred_address: Option<Address>,
    pub all_address: Option<Vec<Address>>,
    pub public_key: Option<PublicKey>
}

impl AccountKeySource {
    pub fn definitely_not_hot(&self) -> bool {
        self.request_type != Some(XPubLikeRequestType::Hot) && self.request_type.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedAddress {
    pub name: String,
    pub address: String,
    pub contact: String,
    pub related_keysource: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct WatchedAddress {
    pub name: String,
    pub address: String,
    pub alert_all: bool,
    pub alert_outgoing: bool
}


#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct ServerTrustRatingLabels {
    pub peer_id_index: i64,
    pub labels: Vec<TrustRatingLabel>,
    pub environment: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct Identity {
    pub name: String,
    pub peer_id_index: i64,
    pub xpub_name: String
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct Contact {
    pub name: String,
    pub peer_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredMnemonic {
    pub name: String,
    pub mnemonic: String,
    pub passphrase: Option<String>,
    pub persist_disk: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredPrivateKey {
    pub name: String,
    pub key_hex: String,
}

/// For data that is lightweight enough to be stored directly in config,
/// somewhat ephermal, and not directly connected to chain data, such as external
/// wallet integrations data before it appears in the network.
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct InternalStoredData {
    pub pending_external_swaps: Option<Vec<PendingExternalSwapInfo>>
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct PendingExternalSwapInfo {
    // Could replace this potentially with specific class, but it's mostly the same.
    pub external_tx: ExternalTimedTransaction,
    pub party_id: PublicKey,
    pub destination_currency: SupportedCurrency,
    pub expected_amount: CurrencyAmount
}

// TODO: Change server to new format
// TODO: Make all values optional for config loader.
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalStoredState {
    pub deploy: Option<Deployment>,
    #[deprecated]
    pub servers: Option<Vec<ServerOldFormat>>,
    pub keys: Option<Vec<AccountKeySource>>,
    pub trust: Option<Vec<ServerTrustRatingLabels>>,
    pub saved_addresses: Option<Vec<SavedAddress>>,
    pub contacts: Option<Vec<Contact>>,
    pub watched_address: Option<Vec<Address>>,
    pub email_alert_config: Option<String>,
    pub identities: Option<Vec<Identity>>,
    pub mnemonics: Option<Vec<StoredMnemonic>>,
    pub private_keys: Option<Vec<StoredPrivateKey>>,
    pub internal_stored_data: Option<InternalStoredData>
}

impl LocalStoredState {

    pub fn clear_sensitive(&mut self) {
        self.mnemonics = self.mnemonics.clone().map(|mnemonics| {
            mnemonics.iter().filter(|mnemonic| {
                mnemonic.persist_disk.unwrap_or(true)
            }).map(|d| d.clone()).collect_vec()
        });
        self.keys = self.keys.clone().map(|x| {
            let vec = x.iter().filter(|xpubs| {
                !xpubs.skip_persist.unwrap_or(false)
            }).map(|d| d.clone()).collect_vec();
            vec
        });
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