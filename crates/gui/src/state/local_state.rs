use crate::components::tx_progress::PreparedTransaction;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AddressInfo, NetworkEnvironment, PublicKey, SupportedCurrency, Transaction};
use redgold_schema::{error_info, RgResult};
use std::collections::HashMap;
use eframe::egui;
use eframe::egui::Ui;
use itertools::Itertools;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::Channel;
use redgold_schema::conf::local_stored_state::{AccountKeySource, Identity, LocalStoredState, StoredMnemonic, StoredPrivateKey};
use redgold_schema::conf::node_config::NodeConfig;
use crate::airgap::signer_window::AirgapSignerWindow;
use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::{GuiDepends, HardwareSigningInfo, TransactionSignInfo};
use crate::functionality::qr_window::{QrShowState, QrState};
use crate::tab::address_tab::AddressTabState;
use crate::tab::deploy::deploy_state::ServersState;
use crate::tab::home::HomeState;
use crate::tab::keys::key_source_sel::key_source;
use crate::tab::keys::keygen::{KeyTabState, KeygenState};
use crate::tab::keys::save_key_window;
use crate::tab::portfolio::port_view::PortfolioTabState;
use crate::tab::settings_tab::SettingsState;
use crate::tab::tabs::Tab;
use crate::tab::transact::swap::SwapState;
use crate::tab::transact::wallet_state::WalletState;

#[derive(Clone, Debug)]
pub enum LocalStateUpdate {
    PricesPartyInfoAndDelta(PricesPartyInfoAndDeltaInitialQuery),
    HardwareSignedInternalTransaction(Transaction),
    BalanceUpdates(BalanceAddressInfoUpdate),
    // TODO: Remove this in favor of unification with other transaction handlers
    SwapResult(RgResult<PreparedTransaction>),
    RequestHardwareRefresh
}

#[derive(Clone, Debug)]
pub struct PricesPartyInfoAndDeltaInitialQuery {
    pub prices: HashMap<SupportedCurrency, f64>,
    pub party_info: HashMap<PublicKey, PartyInternalData>,
    pub delta_24hr: HashMap<SupportedCurrency, f64>,
    pub daily_one_year: HashMap<SupportedCurrency, Vec<(i64, f64)>>,
    pub on_network: NetworkEnvironment
}

#[derive(Clone, Debug)]
pub struct BalanceAddressInfoUpdate {
    pub balances: HashMap<SupportedCurrency, f64>,
    pub address_info: Option<AddressInfo>
}

// #[derive(Clone)]
pub struct LocalState<E> where E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    pub active_tab: Tab,
    pub data: HashMap<NetworkEnvironment, DataQueryInfo<E>>,
    pub node_config: NodeConfig,
    pub home_state: HomeState,
    pub server_state: ServersState,
    pub current_time: i64,
    pub keygen_state: KeygenState,
    pub wallet: WalletState,
    pub qr_state: QrState,
    pub qr_show_state: QrShowState,
    // pub identity_state: IdentityState,
    pub settings_state: SettingsState,
    pub address_state: AddressTabState,
    // pub otp_state: OtpState,
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


pub fn hot_header<E, G>(ls: &mut LocalState<E>, ui: &mut Ui, _ctx: &egui::Context, g: &G
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone, G: GuiDepends + 'static + Sync + Send + Clone {

    save_key_window::save_key_window(ui, ls, _ctx, g);

    key_source(ui, ls, g);

    let state = &mut ls.wallet;

    let check = state.mnemonic_or_key_checksum.clone();
    ui.label(format!("Hot Wallet Checksum: {check}"));

    if state.public_key.is_none() {
        state.update_hot_mnemonic_or_key_info(g);
    }

}


pub(crate) fn init_state(state: &mut WalletState) {
    // TODO: From constant or function for account zero.
    state.derivation_path = "m/44'/16180'/0'/0/0".to_string();
    state.xpub_derivation_path = "m/44'/16180'/0'".to_string();
    state.public_key = None;
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
        let hex = G::private_at(self.wallet.hot_mnemonic(g), string.clone()).unwrap();
        // let kp = self.wallet.hot_mnemonic(g).keypair_at(string).unwrap();
        // let hex = kp.to_private_hex();
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
