#![allow(dead_code)]

use std::collections::HashMap;
use std::{env, fs};
use std::fmt::format;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Once};
use eframe::egui::widgets::TextEdit;
use eframe::egui::{Align, ScrollArea, TextStyle};
use eframe::egui;
use flume::Sender;
use itertools::Itertools;
use tracing::{error, info};
use redgold_schema::{error_info, RgResult};

use crate::util::sym_crypt;
// 0.8
// use crate::gui::image_load::TexMngr;
use crate::gui::{top_panel, ClientApp};
use crate::util;
use rand::Rng;
use rocket::form::validate::Contains;
use serde::{Deserialize, Serialize};
use redgold_gui::dependencies::gui_depends::{HardwareSigningInfo, MnemonicWordsAndPassphrasePath, TransactionSignInfo};
// impl NetworkStatusInfo {
//     pub fn default_vec() -> Vec<Self> {
//         NetworkEnvironment::status_networks().iter().enumerate().map()
//     }
// }
use crate::gui::components::tx_signer::{TxBroadcastProgress, TxSignerProgress};

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
pub struct LocalState {
    pub(crate) active_tab: Tab,
    pub data: DataQueryInfo<ExternalNetworkResourcesImpl>,
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
    pub address_state: AddressState,
    pub otp_state: OtpState,
    pub local_stored_state: LocalStoredState,
    pub updates: Channel<StateUpdate>,
    pub keytab_state: KeyTabState,
    pub is_mac: bool,
    pub is_linux: bool,
    pub is_wasm: bool,
    pub swap_state: SwapState,
    pub external_network_resources: ExternalNetworkResourcesImpl,
    pub price_map_usd_pair: HashMap<SupportedCurrency, f64>,
    pub party_data: HashMap<PublicKey, PartyInternalData>,
    pub first_party: Option<PartyInternalData>,
    pub airgap_signer: AirgapSignerWindow,
    pub persist_requested: bool
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
    fn process_updates(&mut self);
    fn price_map_incl_rdg(&self) -> HashMap<SupportedCurrency, f64>;
    fn hot_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo;
    // fn encrypt(&self, str: String) -> Vec<u8>;
    // fn decrypt(&self, data: &[u8]) -> Vec<u8>;
    // fn accept_passphrase(&mut self, pass: String);
    // fn hash_password(&mut self) -> [u8; 32];
    // fn store_password(&mut self);
    fn cold_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo;
}

impl LocalStateAddons for LocalState {
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
                self.settings_state.lss_serialized = self.local_stored_state.json_or();
            }
            _ => {}
        }
    }
    fn add_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool) {
        self.updates.sender.send(StateUpdate {
            update: Box::new(
                move |lss: &mut LocalState| {
                    lss.upsert_mnemonic(StoredMnemonic {
                        name: name.clone(),
                        mnemonic: mnemonic.clone(),
                        passphrase: None,
                        persist_disk: Some(persist_disk),
                    });
                })
        }).unwrap();
    }
    fn add_with_pass_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool, passphrase: Option<String>) {
        let pass = passphrase.clone();
        self.updates.sender.send(StateUpdate {
            update: Box::new(
                move |lss: &mut LocalState| {
                    let mut m = StoredMnemonic {
                        name: name.clone(),
                        mnemonic: mnemonic.clone(),
                        passphrase: pass.clone(),
                        persist_disk: Some(persist_disk),
                    };
                    lss.upsert_mnemonic(m);
                })
        }).unwrap();
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


    fn process_updates(&mut self) {
        match self.updates.recv_while() {
            Ok(updates) => {
                for mut update in updates {
                    (update.update)(self);
                }
            }
            Err(e) => { error!("Error receiving updates: {}", e.json_or()) }
        }
    }

    fn price_map_incl_rdg(&self) -> HashMap<SupportedCurrency, f64> {
        let mut price_map = self.price_map_usd_pair.clone();
        let cpp = self.first_party.as_ref().and_then(|p| p.party_events.as_ref())
            .and_then(|pe| pe.central_prices.get(&SupportedCurrency::Ethereum))
            .map(|c| c.min_bid_estimated.clone())
            .unwrap_or(100.0);
        price_map.insert(SupportedCurrency::Redgold, cpp);
        price_map
    }

    fn hot_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo {
        // TODO: Need to migrate WordsPass to schema for trait impls.
        let kp = self.wallet.hot_mnemonic().keypair_at(self.keytab_state.derivation_path_xpub_input_account.derivation_path()).unwrap();
        let hex = kp.to_private_hex();
        TransactionSignInfo::PrivateKey(hex)
    }

    fn cold_transaction_sign_info<G>(&self, g: &G) -> TransactionSignInfo {
        // TODO: Need to migrate WordsPass to schema for trait impls.
        let path = self.keytab_state.derivation_path_xpub_input_account.derivation_path();
        let mut info = HardwareSigningInfo::default();
        info.path = path;
        TransactionSignInfo::ColdOrAirgap(info)
    }

    //
    // fn encrypt(&self, str: String) -> Vec<u8> {
    //     return sym_crypt::encrypt(
    //         str.as_bytes(),
    //         &self.session_password_hashed.unwrap(),
    //         &self.iv,
    //     )
    //     .unwrap();
    // }
    //
    // fn decrypt(&self, data: &[u8]) -> Vec<u8> {
    //     return sym_crypt::decrypt(data, &self.session_password_hashed.unwrap(), &self.iv).unwrap();
    // }
    //
    // fn accept_passphrase(&mut self, pass: String) {
    //     let encrypted = self.encrypt(pass);
    //     self.stored_passphrase = encrypted;
    // } // https://www.quora.com/Is-it-useful-to-multi-hash-like-10-000-times-a-password-for-an-anti-brute-force-encryption-algorithm-Do-different-challenges-exist
    //
    // fn hash_password(&mut self) -> [u8; 32] {
    //     let mut vec = self.password_entry.as_bytes().to_vec();
    //     vec.extend(self.session_salt.to_vec());
    //     return dhash_vec(&vec);
    // }
    // fn store_password(&mut self) {
    //     self.session_password_hashed = Some(self.hash_password());
    // }
}


use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter;
use surf::http::security::default;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common_no_wasm::data_folder_read_ext::EnvFolderReadExt;
use redgold_schema::structs::{ErrorInfo, PublicKey, SupportedCurrency};
use redgold_schema::conf::node_config::NodeConfig; // 0.17.1


use redgold_data::data_store::DataStore;
use redgold_gui::airgap::signer_window::AirgapSignerWindow;
use redgold_gui::components::tx_progress::{PreparedTransaction, TransactionProgressFlow};
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::dependencies::extract_public::ExtractorPublicKey;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_gui::tab::deploy::deploy_state::{ServerStatus, ServersState};
// 0.8
// use crate::gui::image_load::TexMngr;
use redgold_gui::tab::home;
use redgold_gui::tab::tabs::Tab;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::util::dhash_vec;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::helpers::easy_json::EasyJson;
use crate::core::internal_message::{new_channel, Channel};
use redgold_gui::tab::home::HomeState;
use redgold_schema::conf::local_stored_state::{AccountKeySource, Identity, LocalStoredState, StoredMnemonic, StoredPrivateKey, XPubLikeRequestType};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::util::lang_util::AnyPrinter;
use crate::gui::components::swap::{SwapStage, SwapState};
use crate::gui::tabs::address_tab::AddressState;
use crate::gui::tabs::identity_tab::IdentityState;
use crate::gui::tabs::otp_tab::{otp_tab, OtpState};
use crate::gui::tabs::server_tab;
use redgold_gui::tab::keys::keygen::KeygenState;
use crate::gui::tabs::keys::keys_tab::{keys_tab, KeyTabState};
use crate::gui::tabs::settings_tab::{settings_tab, SettingsState};
use crate::gui::tabs::transact::hot_wallet::init_state;
use crate::gui::tabs::transact::wallet_tab::{wallet_screen, StateUpdate, WalletState};
use crate::gui::qr_window::{qr_show_window, qr_window, QrShowState, QrState};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node_config::{ApiNodeConfig, DataStoreNodeConfig, EnvDefaultNodeConfig};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::HashType::Transaction;

static INIT: Once = Once::new();

// /// Setup function that is only run once, even if called multiple times.
// pub fn init_logger_once() {
//     INIT.call_once(|| {
//         init_logger();
//     });
// }

pub fn app_update<G>(app: &mut ClientApp<G>, ctx: &egui::Context, _frame: &mut eframe::Frame) where G: GuiDepends + Clone + Send + 'static {
    // let logo = app.logo.clone();
    let mut g = app.gui_depends.clone();
    let local_state = &mut app.local_state;
    g.set_network(&local_state.node_config.network);
    if local_state.persist_requested {
        let mut c = g.get_config();
        c.local = Some(local_state.local_stored_state.clone());
        g.set_config(&c);
        local_state.persist_requested = false;
    }

    // TODO: Replace with config query and check.
    INIT.call_once(|| {
        let amt = if local_state.is_mac {
            2.5
        } else if local_state.is_linux {
            1.8
        } else {
            2.0
        };
        ctx.set_pixels_per_point(amt);
    });

    local_state.current_time = util::current_time_millis_i64();
    // Continuous mode
    ctx.request_repaint();

    local_state.process_updates();

    // let mut style: egui::Style = (*ctx.style()).clone();
    // style.visuals.widgets.
    //style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    // ctx.set_style(style);
    // Examples of how to create different panels and windows.
    // Pick whichever suits you.
    // Tip: a good default choice is to just keep the `CentralPanel`.
    // For inspiration and more examples, go to https://emilk.github.io/egui

    // TODO: Change this to lock screen state transition, also enable it only based on a lock button
    // if local_state.session_password_hashed.is_none() || local_state.session_locked {
    //     update_lock_screen(app, ctx, frame);
    //     return;
    // }

    top_panel::render_top(ctx, local_state);

    // let img = logo;
    // let texture_id = img.texture_id(ctx);

    let mut changed_tab: Option<Tab> = None;

    egui::SidePanel::left("side_panel")
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(54f32);

            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::default()),
                |ui| {
                    ui.add_space(10f32);
                    // ui.image(texture_id); //, size);
                    let image = egui::Image::new(egui::include_image!("../resources/images/historical/design_one/logo_orig_crop.png"));
                    // image.load_for_size(ctx, size).expect("works");
                    ui.add(
                        image
                        // egui::Image::new("https://picsum.photos/seed/1.759706314/1024").rounding(10.0),
                    );

                    ui.style_mut().override_text_style = Some(TextStyle::Heading);

                    ui.style_mut().spacing.item_spacing.y = 5f32;
                    ui.add_space(10f32);
                    for tab_i in Tab::iter() {
                        let tab_str = format!("{:?}", tab_i);
                        if ui.button(tab_str).clicked() {
                            local_state.active_tab = tab_i.clone();
                            local_state.process_tab_change(tab_i.clone());
                            changed_tab = Some(tab_i.clone());
                        }
                    }
                },
            );
        });

    let has_changed_tab = changed_tab.is_some();

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        match local_state.active_tab {
            Tab::Home => {
                let pks = local_state.local_stored_state.extract(&g);
                local_state.home_state.home_screen(
                    ui, ctx, &g, &local_state.external_network_resources, &local_state.data,
                    &local_state.node_config, pks.iter().collect_vec(), local_state.current_time,
                    &local_state.local_stored_state
                );
            }
            Tab::Keys => {
                keys_tab(ui, ctx, local_state, has_changed_tab, &g);
            }
            Tab::Settings => {
                settings_tab(ui, ctx, local_state);
            }
            Tab::Ratings => {}
            Tab::Deploy => {
                ScrollArea::vertical().id_source("wtf").show(ui, |ui| {
                    server_tab::servers_tab(
                        ui,
                        ctx,
                        &mut local_state.server_state,
                        &g,
                        &local_state.node_config,
                        local_state.wallet.hot_mnemonic().words,
                        local_state.wallet.hot_mnemonic().passphrase,
                    );
                });
            }
            Tab::Transact => {
                wallet_screen(ui, ctx, local_state, has_changed_tab, &g, &local_state.data.clone());
            }
            Tab::Identity => {
                crate::gui::tabs::identity_tab::identity_tab(ui, ctx, local_state);
            }
            Tab::Address => {
                crate::gui::tabs::address_tab::address_tab(ui, ctx, local_state);
            },
            Tab::OTP => {
                otp_tab(ui, ctx, local_state);
            }
            Tab::Airgap => {
                // local_state.wallet.active_hot_mnemonic
                let x = MnemonicWordsAndPassphrasePath{
                    words: local_state.keygen_state.mnemonic_window_state.words.clone(),
                    passphrase: local_state.keygen_state.mnemonic_window_state.passphrase.clone(),
                    path: None,
                };
                let info = TransactionSignInfo::Mnemonic(x);
                local_state.airgap_signer.interior_view(ui, &g, Some(&info));
            }
            _ => {}
        }
        ui.with_layout(egui::Layout::top_down(Align::BOTTOM), |ui| {
            egui::warn_if_debug_build(ui)
        });
    });

    qr_window(ctx, local_state);
    qr_show_window(ctx, local_state);

}


#[ignore]
#[tokio::test]
async fn debug() {
    let nc = NodeConfig::dev_default().await;
    let party_data = nc.api_rg_client().party_data().await.log_error().unwrap();
    let p = party_data.into_iter().next().unwrap().1;
    let p = p.party_events.unwrap().central_prices.get(&SupportedCurrency::Ethereum).cloned().unwrap();
    let amt = (0.04143206 * 1e8) as u64;
    let result = p.dummy_fulfill(amt, false, &nc.network, SupportedCurrency::Ethereum);
    println!("Result: {:?}", result);
}

impl LocalState {

}