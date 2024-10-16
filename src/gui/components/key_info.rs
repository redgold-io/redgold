use eframe::egui::{Color32, RichText, Ui};
use tracing::info;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::KeyPair;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::XpubWrapper;
use redgold_schema::{error_info, RgResult};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{NetworkEnvironment, PublicKey, SupportedCurrency};
use crate::gui::app_loop::LocalState;
use redgold_gui::common::{data_item, data_item_hyperlink, editable_text_input_copy, valid_label};
use crate::gui::components::explorer_links::{rdg_explorer, rdg_explorer_links};

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
    pub derivation_path: String,
    pub secret_key: Option<String>,
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
            secret_key: None,
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
            self.public_key = pk.hex();
            self.address = pk.address()
                .and_then(|a| a.render_string())
                .unwrap_or("Address failure".to_string());
            self.btc_address = pk.to_bitcoin_address(&self.network).unwrap_or("".to_string());
            self.eth_address = pk.to_ethereum_address().unwrap_or("".to_string());

            // info!("Public key found in update_public_key_info {}", self.btc_address);
        } else {
            // info!("No public key found in update_public_key_info");
        }
    }

    pub fn update_fields(&mut self,
                         network_environment: &NetworkEnvironment,
                         derivation_path: String,
                         key: GuiKey
    ) {
        // info!("Updating fields called");
        self.network = network_environment.clone();
        self.derivation_path = derivation_path;
        self.key = Some(key);
        self.update_public_key_info();
    }

    pub fn view(&mut self, ui: &mut Ui, option: Option<PublicKey>, environment: NetworkEnvironment) {

        let links = option.map(|pk| rdg_explorer_links(&environment, &pk)).unwrap_or_default();
        let rdg_link = links.get(&SupportedCurrency::Redgold);
        let btc_link = links.get(&SupportedCurrency::Bitcoin);
        let eth_link = links.get(&SupportedCurrency::Ethereum);

        if self.key.is_some() {
            data_item(ui, "Public Key Hex", self.public_key.clone());
            if let Some(r) = rdg_link {
                data_item_hyperlink(ui, "RDG Address", self.address.clone(), r.clone());
            } else {
                data_item(ui, "RDG Address", self.address.clone());
            }
            ui.horizontal(|ui| {
                if let Some(b) = btc_link {
                    data_item_hyperlink(ui, "BTC Address", self.btc_address.clone(), b.clone());
                } else {
                    data_item(ui, "BTC Address", self.btc_address.clone());
                }
                if let Some(e) = eth_link {
                    data_item_hyperlink(ui, "ETH Address", self.eth_address.clone(), e.clone());
                } else {
                    data_item(ui, "ETH Address", self.eth_address.clone());
                }
            });
        }
    }

}


pub fn extract_gui_key(ls: &mut LocalState) -> GuiKey {
    ls.wallet.active_hot_private_key_hex
        .as_ref().map(|x| GuiKey::DirectPrivateKey(x.clone()))
        .unwrap_or(GuiKey::Mnemonic(ls.wallet.hot_mnemonic()))
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
    let xpub = ls.local_stored_state.xpubs.as_ref().and_then(|x| x.iter().find(|x| x.name == ls.wallet.selected_xpub_name));
    if let Some(xpub) = xpub {
        let gui_key = GuiKey::XPub(xpub.xpub.clone());
        ls.keytab_state.xpub_key_info.update_fields(
            &ls.node_config.network,
            ls.keytab_state.derivation_path_xpub_input_account.derivation_path(),
            gui_key
        );
    }

}
