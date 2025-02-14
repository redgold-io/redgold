use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use redgold_schema::constants::default_node_internal_derivation_path;
use redgold_schema::keys::words_pass::WordsPass;
use crate::components::account_deriv_sel::AccountDerivationPathInputState;
use crate::components::derivation_path_sel::DerivationPathInputState;
use crate::dependencies::gui_depends::GuiDepends;
use crate::tab::keys::key_info::KeyInfo;
use crate::tab::keys::xpub_req::RequestXpubState;



impl KeyTabState {
    pub fn new<G>(g: &G) -> Self where G: GuiDepends {
        KeyTabState {
            keygen_subtab: KeygenSubTab::Manage,
            subsubtab: KeygenSubSubTab::XPubs,
            show_private_key_window: false,
            show_xpub: false,
            key_derivation_path_input: Default::default(),
            derivation_path_xpub_input_account: Default::default(),
            request_xpub: Default::default(),
            keys_key_info: KeyInfo::new(g),
            xpub_key_info: KeyInfo::new(g),
            save_xpub_account_name: "".to_string(),
        }
    }
}

#[derive(Debug, EnumIter, Clone, Serialize, Deserialize, EnumString)]
#[repr(i32)]
pub enum KeygenSubTab {
    Manage,
    Generate,
}

#[derive(Debug, EnumIter, Clone, Serialize, Deserialize, EnumString)]
#[repr(i32)]
pub enum KeygenSubSubTab {
    Keys,
    XPubs
}

#[derive(Clone)]
pub struct KeyTabState {
    pub keygen_subtab: KeygenSubTab,
    pub subsubtab: KeygenSubSubTab,
    pub show_private_key_window: bool,
    pub show_xpub: bool,
    pub key_derivation_path_input: DerivationPathInputState,
    pub derivation_path_xpub_input_account: AccountDerivationPathInputState,
    pub request_xpub: RequestXpubState,
    pub keys_key_info: KeyInfo,
    pub xpub_key_info: KeyInfo,
    pub save_xpub_account_name: String
}


#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Clone, EnumString)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum KeyDerivation {
    DoubleSha256,
    Argon2d,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Clone, EnumString)]
pub enum Rounds {
    TenK,
    OneM,
    TenM,
    Custom
}

// TODO: implement a passphrase checksum as well.
// Recalculate these values on change of passphrase
#[derive(Clone)]
pub struct MnemonicWindowState {
    pub open: bool,
    pub words: String,
    pub label: String,
    pub bitcoin_p2wpkh_84: String,
    pub ethereum_address_44: String,
    pub words_checksum: String,
    pub seed_checksum: Option<String>,
    pub passphrase: Option<String>,
    pub redgold_node_address: String,
    pub redgold_hardware_default_address: String,
    pub passphrase_input: String,
    pub passphrase_input_show: bool,
    pub requires_reset: bool,
    pub hd_path: String,
    pub private_key_hex: String,
    pub calc_private_key_hex: bool,
    pub generation_time_seconds: String,
    pub exe_checksum: String,
    pub save_name: String,
    pub persist_disk: bool,
    pub set_hot_mnemonic: bool
}

impl MnemonicWindowState {

    pub(crate) fn get_private_key_hex<G>(&self, g: &G) -> String where G: GuiDepends {
        // self.mnemonic_words().private_hex(self.hd_path.clone()).unwrap_or("err".to_string())
        G::private_at(self.words_pass(), self.hd_path.clone()).unwrap_or("err".to_string())
    }

    fn words_pass(&self) -> WordsPass {
        let w = WordsPass::new(
            self.words.clone(), self.passphrase.clone()
        );
        w
    }

    pub(crate) fn set_words_from_passphrase<G>(&mut self, g: &G) where G: GuiDepends {
        let passphrase = self.passphrase.clone();
        let wp = WordsPass::new(
            self.words.clone(), passphrase.clone()
        );
        let md = G::words_pass_metadata(wp.clone());

        self.bitcoin_p2wpkh_84 = md.btc_84h_0h_0h_0_0_address;
        self.ethereum_address_44 = md.eth_44h_60h_0h_0_0_address;
        self.words_checksum = G::checksum_words(wp.clone()).unwrap();
        self.seed_checksum = G::seed_checksum(wp.clone()).ok();

        self.redgold_node_address= G::public_at(wp.clone(), default_node_internal_derivation_path(0))
            .unwrap().address().unwrap().render_string().unwrap();
        let hw_addr = G::public_at(wp.clone(), "m/44/0/50/0/0")
            .unwrap().address().unwrap().render_string().unwrap();
        self.redgold_hardware_default_address = hw_addr;
    }

    pub(crate) fn set_words<G>(&mut self, words: impl Into<String>, label: impl Into<String>, g: &G) where G: GuiDepends {
        self.open = true;
        self.words = words.into();
        self.label = label.into();
        self.set_words_from_passphrase(g);
    }
}


#[derive(Clone)]
pub struct GenerateMnemonicState {
    pub random_input_mnemonic: String,
    pub random_input_requested: bool,
    pub password_input: String,
    pub show_password: bool,
    pub num_rounds: String,
    pub toggle_concat_password: bool,
    pub toggle_show_metadata: bool,
    pub num_modular_passwords_input: String,
    pub num_modular_passwords: u32,
    pub modular_passwords: Vec<String>,
    pub concat_password: String,
    pub metadata_fields: Vec<String>,
    pub key_derivation: KeyDerivation,
    pub rounds_type: Rounds,
    pub salt_words: String,
    pub m_cost_input: String,
    pub p_cost_input: String,
    pub m_cost: Option<u32>,
    pub p_cost: Option<u32>,
    pub t_cost: Option<u32>,
    pub t_cost_input: String,
}

impl GenerateMnemonicState {
    pub fn compound_passwords(&mut self) {
        let mod_join = self.modular_passwords.iter().join("");
        let metadata_join = self.metadata_fields.iter()
            .map(|s| s.to_uppercase()).join("");
        self.concat_password = format!("{}{}", mod_join, metadata_join);
    }
}

#[derive(Clone)]
pub struct KeygenState {
    pub mnemonic_window_state: MnemonicWindowState,
    pub generate_mnemonic_state: GenerateMnemonicState,
}

impl KeygenState {
    pub fn new(exe_checksum: String) -> Self {
        Self {
            mnemonic_window_state: MnemonicWindowState {
                open: false,
                words: "".to_string(),
                label: "".to_string(),

                bitcoin_p2wpkh_84: "".to_string(),
                ethereum_address_44: "".to_string(),

                words_checksum: "".to_string(),
                seed_checksum: None,
                passphrase: None,
                redgold_node_address: "".to_string(),
                redgold_hardware_default_address: "".to_string(),
                passphrase_input: "".to_string(),
                passphrase_input_show: false,
                requires_reset: false,
                hd_path: "m/44'/5555'/0'/0/0".to_string(),
                private_key_hex: "".to_string(),
                calc_private_key_hex: false,
                generation_time_seconds: "".to_string(),
                exe_checksum,
                save_name: "keygen".to_string(),
                persist_disk: false,
                set_hot_mnemonic: false
            },
            generate_mnemonic_state: GenerateMnemonicState {
                random_input_mnemonic: "".to_string(),
                random_input_requested: false,
                password_input: "".to_string(),
                show_password: false,
                num_rounds: "10000".to_string(),
                toggle_concat_password: false,
                toggle_show_metadata: false,
                num_modular_passwords_input: "6".to_string(),
                num_modular_passwords: 6,
                modular_passwords: (0..6).map(|_| "".to_string()).collect_vec(),
                concat_password: "".to_string(),
                metadata_fields: (0..4).map(|_| "".to_string()).collect_vec(),
                key_derivation: KeyDerivation::Argon2d,
                rounds_type: Rounds::TenK,
                salt_words: "".to_string(),
                m_cost_input: "65536".to_string(),
                p_cost_input: "2".to_string(),
                t_cost_input: "10".to_string(),
                m_cost: Some(65536),
                p_cost: Some(2),
                t_cost: Some(10),
            },
        }
    }
}