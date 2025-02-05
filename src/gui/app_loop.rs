#![allow(dead_code)]

use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::Channel;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::structs::{NetworkEnvironment, PublicKey};
use std::collections::HashMap;
use strum::IntoEnumIterator;
// 0.17.1


use crate::gui::qr_window::{QrShowState, QrState};
use crate::gui::tabs::identity_tab::IdentityState;
use crate::gui::tabs::keys::keys_tab::KeyTabState;
use crate::gui::tabs::otp_tab::OtpState;
use crate::gui::tabs::transact::hot_wallet::init_state;
use redgold_gui::airgap::signer_window::AirgapSignerWindow;
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::state::local_state::LocalStateUpdate;
use redgold_gui::tab::address_tab::AddressTabState;
use redgold_gui::tab::deploy::deploy_state::ServersState;
use redgold_gui::tab::home::HomeState;
use redgold_gui::tab::keys::keygen::KeygenState;
use redgold_gui::tab::portfolio::port_view::PortfolioTabState;
use redgold_gui::tab::settings_tab::SettingsState;
use redgold_gui::tab::tabs::Tab;
use redgold_gui::tab::transact::swap::SwapState;
use redgold_gui::tab::transact::wallet_state::WalletState;
use redgold_keys::xpub_wrapper::XpubWrapper;
use redgold_schema::conf::local_stored_state::{AccountKeySource, Identity, LocalStoredState, StoredMnemonic, StoredPrivateKey};

use itertools::Itertools;
use redgold_schema::{error_info, RgResult};

use redgold_gui::dependencies::gui_depends::{GuiDepends, HardwareSigningInfo, TransactionSignInfo};
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use rocket::form::validate::Contains;

pub trait PublicKeyStoredState {
    fn public_key(&self, xpub_name: String) -> Option<PublicKey>;
}

impl PublicKeyStoredState for LocalStoredState {
    fn public_key(&self, xpub_name: String) -> Option<PublicKey> {
        let pk = self.keys.as_ref().and_then(|x| x.iter().find(|x| x.name == xpub_name)
            .and_then(|g| XpubWrapper::new(g.xpub.clone()).public_at(0, 0).ok()));
        pk
    }
}


// #[derive(Clone)]
pub struct LocalState<E> where E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    pub(crate) active_tab: Tab,
    pub data: HashMap<NetworkEnvironment, DataQueryInfo<E>>,
    pub node_config: NodeConfig,
    pub home_state: HomeState,
    pub server_state: ServersState,
    pub current_time: i64,
    pub keygen_state: KeygenState,
    pub wallet: WalletState,
    pub qr_state: QrState,
    pub qr_show_state: QrShowState,
    pub identity_state: IdentityState,
    pub settings_state: SettingsState,
    pub address_state: AddressTabState,
    pub otp_state: OtpState,
    pub local_stored_state: LocalStoredState,
    // pub updates: Channel<StateUpdate>,
    pub keytab_state: KeyTabState,
    pub is_mac: bool,
    pub is_linux: bool,
    pub is_wasm: bool,
    pub swap_state: SwapState,
    pub external_network_resources: E,
    pub airgap_signer: AirgapSignerWindow,
    pub persist_requested: bool,
    pub local_messages: Channel<LocalStateUpdate>,
    pub latest_local_messages: Vec<LocalStateUpdate>,
    pub portfolio_tab_state: PortfolioTabState
}

pub trait LocalStateAddons {
    fn process_tab_change(&mut self, p0: Tab);
    fn add_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool);
    fn add_with_pass_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool, passphrase: Option<String>);
    fn persist_local_state_store(&mut self);
    fn add_named_xpubs(&mut self, overwrite_name: bool, new_named: Vec<AccountKeySource>, prepend: bool) -> RgResult<()>;
    fn upsert_identity(&mut self, new_named: Identity) -> ();
    fn upsert_mnemonic(&mut self, new_named: StoredMnemonic) -> ();
    fn upsert_private_key(&mut self, new_named: StoredPrivateKey) -> ();
    // fn process_updates(&mut self);
    fn hot_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo where G: GuiDepends + 'static + Sync + Send + Clone;
    // fn encrypt(&self, str: String) -> Vec<u8>;
    // fn decrypt(&self, data: &[u8]) -> Vec<u8>;
    // fn accept_passphrase(&mut self, pass: String);
    // fn hash_password(&mut self) -> [u8; 32];
    // fn store_password(&mut self);
    fn cold_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo where G: GuiDepends + 'static + Sync + Send + Clone;
}

impl<E> LocalStateAddons for LocalState<E> where E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    fn process_tab_change(&mut self, p0: Tab) {
        match p0 {
            Tab::Home => {}
            Tab::Keys => {
                init_state(&mut self.wallet)
            }
            Tab::Transact => {}
            Tab::Portfolio => {}
            Tab::Identity => {}
            Tab::Contacts => {}
            Tab::Address => {}
            Tab::Deploy => {}
            Tab::Ratings => {}
            Tab::Settings => {
                // self.settings_state.local_ser_config = self.local_stored_state.json_or();
            }
            _ => {}
        }
    }
    fn add_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool) {
        // self.updates.sender.send(StateUpdate {
        //     update: Box::new(
        //         move |lss: &mut LocalState| {
                    self.upsert_mnemonic(StoredMnemonic {
                        name: name.clone(),
                        mnemonic: mnemonic.clone(),
                        passphrase: None,
                        persist_disk: Some(persist_disk),
                    });
        //         })
        // }).unwrap();
    }
    fn add_with_pass_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool, passphrase: Option<String>) {
        let pass = passphrase.clone();
        // self.updates.sender.send(StateUpdate {
        //     update: Box::new(
        //         move |lss: &mut LocalState| {
                    let m = StoredMnemonic {
                        name: name.clone(),
                        mnemonic: mnemonic.clone(),
                        passphrase: pass.clone(),
                        persist_disk: Some(persist_disk),
                    };
                    self.upsert_mnemonic(m);
                // })
        // }).unwrap();
    }



    fn persist_local_state_store(&mut self) {
        /*let store = self.secure_or();
        let mut state = self.local_stored_state.clone();
        state.clear_sensitive();
        tokio::spawn(async move {
            store.config_store.update_stored_state(state).await
        });*/
        self.persist_requested = true;
    }
    fn add_named_xpubs(&mut self, overwrite_name: bool, new_named: Vec<AccountKeySource>, prepend: bool) -> RgResult<()> {
        let new_names = new_named.iter().map(|x| x.name.clone())
            .collect_vec();
        let existing = self.local_stored_state.keys.clone().unwrap_or(vec![]);
        let mut filtered = existing.iter().filter(|x| {
            !new_names.contains(&x.name)
        }).map(|x| x.clone()).collect_vec();
        if filtered.len() != existing.len() && !overwrite_name {
            return Err(error_info("Xpub with name already exists"));
        }
        let mut new_named2 = new_named.clone();
        if !prepend {
            filtered.extend(new_named);
            self.local_stored_state.keys = Some(filtered);
        } else {
            new_named2.extend(filtered);
            self.local_stored_state.keys = Some(new_named2);
        }
        self.persist_local_state_store();
        Ok(())
    }
    fn upsert_identity(&mut self, new_named: Identity) -> () {
        let option = self.local_stored_state.identities.clone().unwrap_or(vec![]);
        let mut updated = option.iter().filter(|x| {
            x.name != new_named.name
        }).map(|x| x.clone()).collect_vec();
        updated.push(new_named);

        self.local_stored_state.identities = Some(updated);
        self.persist_local_state_store();
    }


    fn upsert_mnemonic(&mut self, new_named: StoredMnemonic) -> () {
        let mut updated = self.local_stored_state.mnemonics.as_ref().unwrap_or(&vec![]).iter().filter(|x| {
            x.name != new_named.name
        }).map(|x| x.clone()).collect_vec();
        updated.push(new_named);
        self.local_stored_state.mnemonics = Some(updated);
        self.persist_local_state_store();
    }

    fn upsert_private_key(&mut self, new_named: StoredPrivateKey) -> () {
        let mut updated = self.local_stored_state.private_keys.as_ref().unwrap_or(&vec![]).iter().filter(|x| {
            x.name != new_named.name
        }).map(|x| x.clone()).collect_vec();
        updated.push(new_named);
        self.local_stored_state.private_keys = Some(updated);
        self.persist_local_state_store();
    }
    //
    //
    // fn process_updates(&mut self) {
    //     // match self.updates.recv_while() {
    //     //     Ok(updates) => {
    //     //         for mut update in updates {
    //     //             (update.update)(self);
    //     //         }
    //     //     }
    //     //     Err(e) => { error!("Error receiving updates: {}", e.json_or()) }
    //     // }
    // }


    fn hot_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo where G: GuiDepends + 'static + Sync + Send + Clone {
        // TODO: Need to migrate WordsPass to schema for trait impls.
        let string = self.keytab_state.derivation_path_xpub_input_account.derivation_path_valid_fallback();
        let kp = self.wallet.hot_mnemonic(g).keypair_at(string).unwrap();
        let hex = kp.to_private_hex();
        TransactionSignInfo::PrivateKey(hex)
    }

    fn cold_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo where G: GuiDepends + 'static + Sync + Send + Clone {
        // TODO: Need to migrate WordsPass to schema for trait impls.
        let path = self.keytab_state.derivation_path_xpub_input_account.derivation_path_valid_fallback();
        let mut info = HardwareSigningInfo::default();
        info.path = path;
        TransactionSignInfo::ColdOrAirgap(info)
    }

}

