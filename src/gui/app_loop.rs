#![allow(dead_code)]

use std::env;
use std::fmt::format;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Once};
use eframe::egui::widgets::TextEdit;
use eframe::egui::{Align, ScrollArea, TextStyle};
use eframe::egui;
use itertools::Itertools;
use log::{error, info};
use redgold_schema::{error_info, RgResult};

use crate::util::sym_crypt;
// 0.8
// use crate::gui::image_load::TexMngr;
use crate::gui::{ClientApp, home, top_panel};
use crate::util;
use rand::Rng;
use rocket::form::validate::Contains;
// impl NetworkStatusInfo {
//     pub fn default_vec() -> Vec<Self> {
//         NetworkEnvironment::status_networks().iter().enumerate().map()
//     }
// }

pub trait PublicKeyStoredState {
    fn public_key(&self, xpub_name: String) -> Option<PublicKey>;
}

impl PublicKeyStoredState for LocalStoredState {
    fn public_key(&self, xpub_name: String) -> Option<PublicKey> {
        let pk = self.xpubs.iter().find(|x| x.name == xpub_name)
            .and_then(|g| XpubWrapper::new(g.xpub.clone()).public_at(0, 0).ok());
        pk
    }
}


// #[derive(Clone)]
// #[derive(Clone)]
pub struct LocalState {
    active_tab: Tab,
    session_salt: [u8; 32],
    session_password_hashed: Option<[u8; 32]>,
    session_locked: bool,
    // This is only used by the text box and should be cleared immediately
    password_entry: String,
    // This is only used by the text box and should be cleared immediately
    wallet_passphrase_entry: String,
    // wallet_words_entry: String,
    active_passphrase: Option<String>,
    password_visible: bool,
    show_mnemonic: bool,
    visible_mnemonic: Option<String>,
    // TODO: Encrypt these with session password
    // TODO: Allow multiple passphrase, i'm too lazy for now
    // stored_passphrases: HashMap<String, String>,
    stored_passphrase: Vec<u8>,
    // stored_mnemonics: Vec<String>,
    stored_private_key_hexes: Vec<String>,
    iv: [u8; 16],
    wallet_first_load_state: bool,
    pub node_config: NodeConfig,
    // pub runtime: Arc<Runtime>,
    pub home_state: HomeState,
    pub server_state: ServersState,
    pub current_time: i64,
    pub keygen_state: KeygenState,
    pub wallet_state: WalletState,
    pub qr_state: QrState,
    pub qr_show_state: QrShowState,
    pub identity_state: IdentityState,
    pub settings_state: SettingsState,
    pub address_state: AddressState,
    pub otp_state: OtpState,
    pub ds_env: DataStore,
    pub ds_env_secure: Option<DataStore>,
    pub local_stored_state: LocalStoredState,
    pub updates: Channel<StateUpdate>,
    pub keytab_state: KeyTabState,
    pub is_mac: bool,
    pub is_linux: bool,
    pub swap_state: SwapState
}

impl LocalState {
    pub(crate) fn process_tab_change(&mut self, p0: Tab) {
        match p0 {
            Tab::Home => {}
            Tab::Keys => {
                init_state(&mut self.wallet_state)
            }
            Tab::Transact => {}
            Tab::Portfolio => {}
            Tab::Identity => {}
            Tab::Contacts => {}
            Tab::Address => {}
            Tab::Servers => {}
            Tab::Ratings => {}
            Tab::Settings => {
                self.settings_state.lss_serialized = self.local_stored_state.json_or();
            }
            _ => {

            }
        }
    }
}

impl LocalState {

    pub fn add_mnemonic(&mut self, name: String, mnemonic: String, persist_disk: bool) {
        self.updates.sender.send(StateUpdate {
            update: Box::new(
                move |lss: &mut LocalState| {
                    lss.upsert_mnemonic(StoredMnemonic {
                        name: name.clone(),
                        mnemonic: mnemonic.clone(),
                        persist_disk: Some(persist_disk),
                    });
                })
        }).unwrap();
    }

    pub fn secure_or(&self) -> DataStore {
        self.ds_env_secure.clone().unwrap_or(self.ds_env.clone())
    }
    pub fn send_update<F: FnMut(&mut LocalState) + Send + 'static>(updates: &Channel<StateUpdate>, p0: F) {
        updates.sender.send(StateUpdate{update: Box::new(p0)}).unwrap();
    }

    pub fn persist_local_state_store(&self) {
        let store = self.secure_or();
        let mut state = self.local_stored_state.clone();
        state.clear_sensitive();
        tokio::spawn(async move {
            store.config_store.update_stored_state(state).await
        });
    }
    pub fn add_named_xpubs(&mut self, overwrite_name: bool, new_named: Vec<NamedXpub>, prepend: bool) -> RgResult<()> {
        let new_names = new_named.iter().map(|x| x.name.clone())
            .collect_vec();
        let existing = self.local_stored_state.xpubs.clone();
        let mut filtered = existing.iter().filter(|x| {
            !new_names.contains(&x.name)
        }).map(|x| x.clone()).collect_vec();
        if filtered.len() != existing.len() && !overwrite_name {
            return Err(error_info("Xpub with name already exists"));
        }
        let mut new_named2 = new_named.clone();
        if !prepend {
            filtered.extend(new_named);
            self.local_stored_state.xpubs = filtered;
        } else {
            new_named2.extend(filtered);
            self.local_stored_state.xpubs = new_named2;
        }
        self.persist_local_state_store();
        Ok(())
    }
    pub fn upsert_identity(&mut self, new_named: Identity) -> () {
        let mut updated = self.local_stored_state.identities.iter().filter(|x| {
            x.name != new_named.name
        }).map(|x| x.clone()).collect_vec();
        updated.push(new_named);

        self.local_stored_state.identities = updated;
        self.persist_local_state_store();
    }


    pub fn upsert_mnemonic(&mut self, new_named: StoredMnemonic) -> () {
        let mut updated = self.local_stored_state.mnemonics.as_ref().unwrap_or(&vec![]).iter().filter(|x| {
            x.name != new_named.name
        }).map(|x| x.clone()).collect_vec();
        updated.push(new_named);
        self.local_stored_state.mnemonics = Some(updated);
        self.persist_local_state_store();
    }

    pub fn upsert_private_key(&mut self, new_named: StoredPrivateKey) -> () {
        let mut updated = self.local_stored_state.private_keys.as_ref().unwrap_or(&vec![]).iter().filter(|x| {
            x.name != new_named.name
        }).map(|x| x.clone()).collect_vec();
        updated.push(new_named);
        self.local_stored_state.private_keys = Some(updated);
        self.persist_local_state_store();
    }


    pub fn process_updates(&mut self) {
        match self.updates.recv_while() {
            Ok(updates) => {
                for mut update in updates {
                    (update.update)(self);
                }
            }
            Err(e) => { error!("Error receiving updates: {}", e.json_or()) }
        }
    }
}

#[allow(dead_code)]
impl LocalState {
    pub async fn from(node_config: NodeConfig) -> Result<LocalState, ErrorInfo> {
        let mut node_config = node_config.clone();
        node_config.load_balancer_url = "lb.redgold.io".to_string();
        let iv = sym_crypt::get_iv();
        let ds_env = node_config.data_store_all().await;
        let ds_env_secure = node_config.data_store_all_secure().await;
        let ds_or = ds_env_secure.clone().unwrap_or(ds_env.clone());
        info!("Starting local state with secure_or connection path {}", ds_or.ctx.connection_path.clone());
        let string = ds_or.ctx.connection_path.clone().replace("file:", "");
        info!("ds_or connection path {}", string);
        ds_or.run_migrations_fallback_delete(
            true,
            PathBuf::from(string)
        ).await.expect("migrations");
        // DataStore::run_migrations(&ds_or).await.expect("");
        let hot_mnemonic = node_config.secure_or().all().mnemonic().await.unwrap_or(node_config.mnemonic_words.clone());
        let local_stored_state = ds_or.config_store.get_stored_state().await?;

        let mut ss = crate::gui::tabs::server_tab::ServersState::default();

        ss.csv_edit_path = node_config.clone().secure_data_folder.unwrap_or(node_config.data_folder.clone())
            .all().servers_path().to_str().expect("").to_string();

        // ss.genesis = node_config.opts.development_mode;
        let mut ls = LocalState {
            active_tab: Tab::Home,
            session_salt: random_bytes(),
            session_password_hashed: None,
            session_locked: false,
            password_entry: "".to_string(),
            wallet_passphrase_entry: "".to_string(),
            // wallet_words_entry: "".to_string(),
            active_passphrase: None,
            password_visible: false,
            show_mnemonic: false,
            visible_mnemonic: None,
            stored_passphrase: vec![],
            // stored_passphrases: HashMap::new(),
            // stored_mnemonics: vec![],
            stored_private_key_hexes: vec![],
            iv,
            wallet_first_load_state: true,
            node_config: node_config.clone(),
            // runtime,
            home_state: HomeState::from(),
            server_state: ss,
            current_time: util::current_time_millis_i64(),
            keygen_state: KeygenState::new(
                node_config.clone().executable_checksum.clone().unwrap_or("".to_string())
            ),
            wallet_state: WalletState::new(hot_mnemonic, local_stored_state.xpubs.first()),
            qr_state: Default::default(),
            qr_show_state: Default::default(),
            identity_state: IdentityState::new(),
            settings_state: SettingsState::new(local_stored_state.json_or(),
                                               node_config.data_folder.clone().path.parent().unwrap().to_str().unwrap().to_string(),
                                               node_config.secure_data_folder.clone().unwrap_or(node_config.data_folder.clone())
                                                   .path.parent().unwrap().to_str().unwrap().to_string()
            ),
            address_state: Default::default(),
            otp_state: Default::default(),
            ds_env,
            ds_env_secure,
            local_stored_state,
            updates: new_channel(),
            keytab_state: Default::default(),
            is_mac: env::consts::OS == "macos",
            is_linux: env::consts::OS == "linux",
            swap_state: Default::default(),
        };

        if node_config.opts.development_mode {
            ls.server_state.ops = false;
            ls.server_state.system = false;
            if node_config.network.is_main()  {
                ls.server_state.words_and_id = false;
            }
        }

        let mut new_xpubs = vec![];

        if let Some(df) = node_config.secure_data_folder.as_ref() {
            if let Ok(m) = df.all().mnemonic().await {
                if let Ok(w) = WordsPass::new_validated(m.clone(), None) {
                    let key_name = "secure_df_all".to_string();
                    ls.wallet_state.selected_key_name = key_name.clone();
                    ls.add_mnemonic(key_name.clone(), m, false);
                    if let Ok(xpub) = w.named_xpub(key_name, true) {
                        new_xpubs.push(xpub);
                    }
                }
            }
        }

        if let Ok(m) = node_config.data_folder.all().mnemonic().await {
            if let Ok(w) = WordsPass::new_validated(m.clone(), None) {
                let key_name = "df_all".to_string();
                ls.add_mnemonic(key_name.clone(), m, false);
                if let Ok(xpub) = w.named_xpub(key_name, true) {
                    new_xpubs.push(xpub);
                }
            }
        }

        if let Ok(m) = std::env::var("REDGOLD_TEST_WORDS") {
            if let Ok(w) = WordsPass::new_validated(m.clone(), None) {
                let key_name = "test_words".to_string();
                ls.add_mnemonic(key_name.clone(), m, false);
                if let Ok(xpub) = w.named_xpub(&key_name, true) {
                    new_xpubs.push(xpub);
                }
                let dp_btc_faucet = "m/84'/0'/0'/0/0".to_string();
                if let Ok(xpub) = w.xpub_str(&dp_btc_faucet.as_account_path().expect("acc")) {
                    let mut named = NamedXpub::default();
                    let key_into = key_name.clone();
                    named.name = format!("{}_840", key_into);
                    named.xpub = xpub;
                    named.key_name_source = Some(key_into);
                    named.request_type = Some(XPubRequestType::Hot);
                    named.skip_persist = Some(true);
                    named.derivation_path = dp_btc_faucet.clone();
                    new_xpubs.push(named);
                }
            }
        }

        if !new_xpubs.is_empty() {
            let first_xpub = new_xpubs.get(0).unwrap().clone();
            ls.wallet_state.selected_xpub_name = first_xpub.name.clone();
            ls.add_named_xpubs(true, new_xpubs, true).expect("Adding xpubs");
        }

        // TODO: Add from environment

        Ok(ls)
    }

    fn encrypt(&self, str: String) -> Vec<u8> {
        return sym_crypt::encrypt(
            str.as_bytes(),
            &self.session_password_hashed.unwrap(),
            &self.iv,
        )
        .unwrap();
    }

    fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        return sym_crypt::decrypt(data, &self.session_password_hashed.unwrap(), &self.iv).unwrap();
    }

    pub fn accept_passphrase(&mut self, pass: String) {
        let encrypted = self.encrypt(pass);
        self.stored_passphrase = encrypted;
    } // https://www.quora.com/Is-it-useful-to-multi-hash-like-10-000-times-a-password-for-an-anti-brute-force-encryption-algorithm-Do-different-challenges-exist

    fn hash_password(&mut self) -> [u8; 32] {
        let mut vec = self.password_entry.as_bytes().to_vec();
        vec.extend(self.session_salt.to_vec());
        return dhash_vec(&vec);
    }
    fn store_password(&mut self) {
        self.session_password_hashed = Some(self.hash_password());
    }
}

fn random_bytes() -> [u8; 32] {
    return rand::thread_rng().gen::<[u8; 32]>();
}

use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter;
use redgold_schema::structs::{ErrorInfo, PublicKey};
use redgold_schema::conf::node_config::NodeConfig; // 0.17.1



#[derive(Debug, EnumIter, Clone)]
#[repr(i32)]
pub enum Tab {
    Home,
    Keys,
    Transact,
    Portfolio,
    Identity,
    Contacts,
    Address,
    Servers,
    Ratings,
    Settings,
    OTP,
}

fn update_lock_screen(app: &mut ClientApp, ctx: &egui::Context) {
    let ClientApp { local_state, .. } = app;
    egui::CentralPanel::default().show(ctx, |ui| {
        let layout = egui::Layout::top_down(egui::Align::Center);
        ui.with_layout(layout, |ui| {
            ui.add_space(ctx.available_rect().max.y / 3f32);
            ui.heading("Enter session password");
            ui.add_space(20f32);

            let edit = TextEdit::singleline(&mut local_state.password_entry)
                .password(true)
                .lock_focus(true);
            ui.add(edit).request_focus();
            if ctx.input(|i| { i.key_pressed(egui::Key::Enter)}) {
                if local_state.session_locked {
                    if local_state.session_password_hashed.unwrap() == local_state.hash_password() {
                        local_state.session_locked = false;
                    } else {
                        panic!("Session password state error");
                    }
                } else {
                    local_state.store_password();
                }
                local_state.password_entry = "".to_string();
                ()
            };
            //ui.text_edit_singleline(texts);
        });
    });
}

use redgold_data::data_store::DataStore;
use redgold_keys::util::dhash_vec;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::helpers::easy_json::EasyJson;
use crate::core::internal_message::{Channel, new_channel};
use crate::gui::home::HomeState;
use redgold_schema::local_stored_state::{Identity, LocalStoredState, NamedXpub, StoredMnemonic, StoredPrivateKey, XPubRequestType};
use crate::gui::components::swap::SwapState;
use crate::gui::tabs::address_tab::AddressState;
use crate::gui::tabs::identity_tab::IdentityState;
use crate::gui::tabs::otp_tab::{otp_tab, OtpState};
use crate::gui::tabs::server_tab;
use crate::gui::tabs::keys::keygen_subtab::KeygenState;
use crate::gui::tabs::keys::keys_tab::{keys_tab, KeyTabState};
use crate::gui::tabs::server_tab::{ServersState, ServerStatus};
use crate::gui::tabs::settings_tab::{settings_tab, SettingsState};
use crate::gui::tabs::transact::hot_wallet::init_state;
use crate::gui::tabs::transact::wallet_tab::{StateUpdate, wallet_screen, WalletState};
use crate::gui::qr_window::{qr_show_window, qr_window, QrShowState, QrState};
use crate::infra::deploy::is_windows;
use crate::node_config::DataStoreNodeConfig;

static INIT: Once = Once::new();

// /// Setup function that is only run once, even if called multiple times.
// pub fn init_logger_once() {
//     INIT.call_once(|| {
//         init_logger();
//     });
// }

pub fn app_update(app: &mut ClientApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let ClientApp {
        logo,
        local_state,
    } = app;

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

    let img = logo;
    // let texture_id = img.texture_id(ctx);

    let mut changed_tab: Option<Tab> = None;

    egui::SidePanel::left("side_panel")
        .resizable(false)
        .show(ctx, |ui| {
            // ui.horizontal(|ui| {
            //     ui.label("Write something: ");
            //     ui.text_edit_singleline(label);
            // });
            // ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));

            //https://github.com/emilk/egui/blob/master/egui_demo_lib/src/apps/http_app.rs
            // ui.image(TextureId::default())

            ui.set_max_width(54f32);
            // ui.set_max_width(104f32);

            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::default()),
                |ui| {
                    let scale = 2.0;
                    let size =
                        egui::Vec2::new((img.size()[0] as f32 / scale) as f32, (img.size()[1] as f32 / scale) as f32);
                    // ui.style_mut().spacing.window_padding.y += 20.0f32;
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
                    //
                    // if ui.button("Home").clicked() {
                    //     *tab = Tab::Home;
                    // }
                    for tab_i in Tab::iter() {
                        let tab_str = format!("{:?}", tab_i);
                        if ui.button(tab_str).clicked() {
                            local_state.active_tab = tab_i.clone();
                            local_state.process_tab_change(tab_i.clone());
                            changed_tab = Some(tab_i.clone());
                        }
                    }
                    //
                    // if ui.button("Wallet").clicked() {
                    //     *tab = Tab::Wallet;
                    // }
                    //
                    // if ui.button("Settings").clicked() {
                    //     *tab = Tab::Wallet;
                    // }
                },
            );

            // ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            //     ui.add(
            //         egui::Hyperlink::new("https://github.com/emilk/egui/").text("powered by egui"),
            //     );
            // });
        });

    // if ctx.input().key_pressed(egui::Key::Escape) {
    //     local_state.session_locked = true;
    // }
    let has_changed_tab = changed_tab.is_some();

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        match local_state.active_tab {
            Tab::Home => {
                home::home_screen(ui, ctx, local_state);
            }
            Tab::Keys => {

                keys_tab(ui, ctx, local_state, has_changed_tab);
            }
            Tab::Settings => {
                settings_tab(ui, ctx, local_state);
            }
            Tab::Ratings => {}
            Tab::Servers => {
                ScrollArea::vertical().id_source("wtf").show(ui, |ui| {
                    server_tab::servers_tab(ui, ctx, local_state);
                });
            }
            Tab::Transact => {
                wallet_screen(ui, ctx, local_state, has_changed_tab);
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
            _ => {}
        }
        // ui.hyperlink("https://github.com/emilk/egui_template");
        // ui.add(egui::github_link_file!(
        //     "https://github.com/emilk/egui_template/blob/master/",
        //     "Source code."
        // ));
        ui.with_layout(egui::Layout::top_down(Align::BOTTOM), |ui| {
            egui::warn_if_debug_build(ui)
        });
    });

    qr_window(ctx, local_state);
    qr_show_window(ctx, local_state);

    // sync local data to RDS -- apart from data associated with phrases
    // discuss extra features around confirmation process. p2p negotation, contacts table.
}
