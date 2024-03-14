use eframe::egui::{Color32, RichText, Ui};
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::KeyPair;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::XpubWrapper;
use redgold_schema::{error_info, RgResult};
use redgold_schema::structs::{NetworkEnvironment, PublicKey};
use crate::gui::app_loop::LocalState;
use crate::gui::common::{data_item, editable_text_input_copy, valid_label};


const DEFAULT_DP: &str = "m/44'/0'/50'/0/0";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GuiKey {
    DirectPrivateKey(String),
    XPub(String),
    Mnemonic(WordsPass),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KeyInfo {
    pub key: Option<GuiKey>,
    pub public_key: String,
    pub address: String,
    pub btc_address: String,
    pub eth_address: String,
    pub network: NetworkEnvironment,
    pub derivation_path: String
}


impl Default for KeyInfo {
    fn default() -> Self {
        Self::new()
    }
}
impl KeyInfo {

    pub fn new() -> Self {
        let mut ki = KeyInfo {
            key: None,
            public_key: "".to_string(),
            address: "".to_string(),
            btc_address: "".to_string(),
            eth_address: "".to_string(),
            network: NetworkEnvironment::Dev,
            derivation_path: DEFAULT_DP.to_string(),
        };
        ki.update_public_key_info();
        ki
    }

    pub fn public_key(&self) -> RgResult<PublicKey> {
        if let Some(k) = self.key.as_ref() {
            match k {
                GuiKey::DirectPrivateKey(k) => {
                    KeyPair::from_private_hex(k.clone()).map(|h| h.public_key())
                }
                GuiKey::Mnemonic(m) => {
                    m.public_at(self.derivation_path.clone())
                }
                GuiKey::XPub(xpub) => {
                    let w = XpubWrapper::new(xpub.clone());
                    let public = w.public_at_dp(&self.derivation_path);
                    public
                }
            }
        } else {
            Err(error_info("No key".to_string()))
        }
    }

    fn update_public_key_info(&mut self) {
        if let Ok(pk) = self.public_key() {
            self.public_key = pk.hex_or();
            self.address = pk.address()
                .and_then(|a| a.render_string())
                .unwrap_or("Address failure".to_string());
            self.btc_address = pk.to_bitcoin_address(&self.network).unwrap_or("".to_string());
            self.eth_address = pk.to_ethereum_address().unwrap_or("".to_string());

            info!("Public key found in update_public_key_info {}", self.btc_address);
        } else {
            info!("No public key found in update_public_key_info");
        }
    }

    pub fn update_fields(&mut self,
                         network_environment: &NetworkEnvironment,
                         derivation_path: String,
                         key: GuiKey
    ) {
        info!("Updating fields called");
        self.network = network_environment.clone();
        self.derivation_path = derivation_path;
        self.key = Some(key);
        self.update_public_key_info();
    }

    pub fn view(&mut self, ui: &mut Ui) {
        if self.key.is_some() {
            data_item(ui, "Public Key Hex", self.public_key.clone());
            data_item(ui, "RDG Address", self.address.clone());
            data_item(ui, "BTC Address", self.btc_address.clone());
            data_item(ui, "ETH Address", self.eth_address.clone());
        }
    }

}


pub fn extract_gui_key(ls: &mut LocalState) -> GuiKey {
    ls.wallet_state.active_hot_private_key_hex
        .as_ref().map(|x| GuiKey::DirectPrivateKey(x.clone()))
        .unwrap_or(GuiKey::Mnemonic(ls.wallet_state.hot_mnemonic()))
}


pub fn update_keys_key_info(ls: &mut LocalState) {
    let gui_key = extract_gui_key(ls);
    ls.keytab_state.keys_key_info.update_fields(
        &ls.node_config.network,
        ls.keytab_state.key_derivation_path_input.derivation_path.clone(),
        gui_key
    );
}

pub fn update_xpub_key_info(ls: &mut LocalState) {
    let xpub = ls.local_stored_state.xpubs.iter().find(|x| x.name == ls.wallet_state.selected_xpub_name);
    if let Some(xpub) = xpub {
        let gui_key = GuiKey::XPub(xpub.xpub.clone());
        ls.keytab_state.xpub_key_info.update_fields(
            &ls.node_config.network,
            ls.keytab_state.derivation_path_xpub_input_account.derivation_path(),
            gui_key
        );
    }

}
