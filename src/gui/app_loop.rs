#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crossbeam::atomic::AtomicCell;
use eframe::egui::widgets::TextEdit;
use eframe::egui::{Align, TextStyle, Ui};
use eframe::{egui};
use itertools::Itertools;
use log::{error, info};

use crate::util::sym_crypt;
// 0.8
// use crate::gui::image_load::TexMngr;
use crate::gui::ClientApp;
use crate::util;
use rand::Rng;
use rocket::http::ext::IntoCollection;
use redgold_schema::util::{dhash_vec, mnemonic_builder};

#[derive(Copy, Clone)]
pub struct NetworkStatusInfo{
    network_index: usize,
    network: NetworkEnvironment,
    reachable: bool
    // num_peers: usize,
    // num_transactions: usize,
    // genesis_hash_short: String,
}

// impl NetworkStatusInfo {
//     pub fn default_vec() -> Vec<Self> {
//         NetworkEnvironment::status_networks().iter().enumerate().map()
//     }
// }

pub struct HomeState {
    network_status_info: Arc<AtomicCell<Vec<NetworkStatusInfo>>>,
    last_query_started_time: Option<i64>
}

impl HomeState {
    pub fn from() -> Self {
        Self {
            network_status_info: Arc::new(AtomicCell::new(vec![])),
            last_query_started_time: None,
        }
    }
}

#[derive(Clone)]
pub struct ServerStatus {
    pub ssh_reachable: bool
}

pub struct ServersState {
    needs_update: bool,
    info: Arc<Mutex<Vec<ServerStatus>>>,
    deployment_result_info_box: Arc<Mutex<String>>
}

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
    node_config: NodeConfig,
    runtime: Arc<Runtime>,
    home_state: HomeState,
    server_state: ServersState,
    current_time: i64
}

#[allow(dead_code)]
impl LocalState {
    pub fn from(node_config: NodeConfig, runtime: Arc<Runtime>) -> LocalState {
        let mut node_config = node_config.clone();
        node_config.load_balancer_url = "lb.redgold.io".to_string();
        let iv = sym_crypt::get_iv();
        return LocalState {
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
            node_config,
            runtime,
            home_state: HomeState::from(),
            server_state: ServersState { needs_update: true,
                info: Arc::new(Mutex::new(vec![])),
                deployment_result_info_box: Arc::new(Mutex::new("".to_string())) },
            current_time: util::current_time_millis_i64(),
        };
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
use tokio::runtime::Runtime;
use redgold_schema::servers::Server;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use crate::infra::{deploy, SSH};
use crate::node_config::NodeConfig; // 0.17.1



#[derive(Debug, EnumIter)]
#[repr(i32)]
pub enum Tab {
    Home,
    Wallet,
    Trust,
    Servers,
    Settings,
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

pub async fn query_network_status(
    node_config: NodeConfig,
    result: Arc<AtomicCell<Vec<NetworkStatusInfo>>>
) -> Result<(), ErrorInfo> {

    let mut results = vec![];
    for (i, x) in NetworkEnvironment::status_networks().iter().enumerate() {
        let mut config = node_config.clone();
        config.network = x.clone();
        let mut client = config.lb_client();
        client.timeout = Duration::from_secs(5);
        let res = client.about().await;

        let reachable = match res {
            Ok(a) => {
                info!("Network status query success: {}", crate::schema::json_or(&a));
                true
            }
            Err(e) => {
                error!("Network status query failed: {}", crate::schema::json_or(&e));
                false
            }
        };
        let status = NetworkStatusInfo{
            network_index: i,
            network: x.clone(),
            reachable,
        };
        results.push(status);
    }
    result.store(results.clone());
    let map2 = result.take();
    result.store(map2.clone());
    // info!("Network status: {}", map2.to_string());
    Ok(())
}
use egui_extras::{Column, TableBuilder};
use surf::http::headers::ToHeaderValues;


pub fn text_table(ui: &mut Ui, data: Vec<Vec<String>>) {

    if data.len() == 0 {
        return;
    }

    let headers = data.get(0).expect("").clone();
    let columns = headers.len();

    let text_height = 25.0;
    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(0.0);

    for _ in 0..columns {
        table = table.column(Column::auto());
    };

    table
        .header(text_height, |mut header| {
            for h in headers {
                header.col(|ui| {
                    ui.strong(h);
                });
            }
        }).body(|mut body| {
        body.rows(text_height, data.len() - 1, |row_index, mut row| {
            let row_data = data.get(row_index + 1).expect("value row missing");
            for cell in row_data {
                row.col(|ui| {
                    ui.label(cell);
                });
            }
        });
    });

}

pub fn home_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    ui.heading("Network Status");
    ui.separator();
    let home_state = &mut local_state.home_state;
    let nc2 = local_state.node_config.clone();
    let arc = home_state.network_status_info.clone();
    if home_state.last_query_started_time
        .map(|q| (local_state.current_time - q) > 1000*25)
        .unwrap_or(true) {
        home_state.last_query_started_time = Some(local_state.current_time);
        local_state.runtime.spawn(async move {
            query_network_status(nc2, arc).await
        });
    }
    let query_status_string = home_state.last_query_started_time.map(|q| {
        format!("Queried: {:?} seconds ago", (local_state.current_time - q) / 1000)
    }).unwrap_or("unknown error".to_string());
    ui.label(query_status_string);
    ui.separator();


    // let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
    let num_rows = NetworkEnvironment::status_networks().len();
    let mut network_index = HashMap::new();
    let row_network: Vec<String> = NetworkEnvironment::status_networks()
        .iter().enumerate().map(|(index, x)| {
        network_index.insert(index, x.clone());
        x.to_std_string()
    }).collect_vec();

    // Well this is ridiculous
    // can we change from atomic cell or use some copyable type?
    let status_info = home_state.network_status_info.take();
    home_state.network_status_info.store(status_info.clone());

    let text_height = 20.0;
    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto())
        .column(Column::auto())
        // .column(Column::initial(100.0).range(40.0..=300.0))
        // .column(Column::initial(100.0).at_least(40.0).clip(true))
        // .column(Column::remainder())
        .min_scrolled_height(0.0);

    // info!("Status home loop info: {:?}", status_info.to_string());
    table
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Network");
            });
            header.col(|ui| {
                ui.strong("Status");
            });
        }).body(|mut body| {
        body.rows(text_height, num_rows, |row_index, mut row| {
            let info = status_info.get(row_index).clone();
            row.col(|ui| {
                let option = row_network.get(row_index).clone().expect("row index missing").clone();
                ui.label(option);
            });
            row.col(|ui| {
                let reachable = info.map(|n| match n.reachable {
                    true => {"Online"}
                    false => {"Offline"}
                }).unwrap_or("querying").to_string();
                ui.label(reachable);
            });
        });
    });

}

pub async fn update_server_status(servers: Vec<Server>, mut status: Arc<Mutex<Vec<ServerStatus>>>) {
    let mut results = vec![];

    for server in servers {
        let mut ssh = SSH::from_server(&server);
        let reachable = ssh.verify().is_ok();
        results.push(ServerStatus{ ssh_reachable: reachable});
    };
    let mut guard = status.lock().expect("lock");
    guard.clear();
    guard.extend(results);
}

pub fn servers_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {


    let servers = local_state.node_config.servers.clone();

    if local_state.server_state.needs_update {
        local_state.server_state.needs_update = false;
        local_state.runtime.spawn(
            update_server_status(
                servers.clone(),
        local_state.server_state.info.clone()
            )
        );
    }
    let info = local_state.server_state.info.lock().expect("").to_vec();

    let mut table_rows: Vec<Vec<String>> = vec![];
    table_rows.push(vec![
            "Hostname".to_string(),
            "SSH status".to_string(),
            "Index".to_string(),
            "PeerId Index".to_string(),
        "SSH User".to_string(),
        "SSH Key Path".to_string(),
    ]);

    for (i, server) in servers.iter().enumerate() {
        let status_i = info.get(i);
        let status = status_i.map(|s| match s.ssh_reachable {
            true => {"Online"}
            false => {"Offline"}
        }).unwrap_or("querying").to_string();
        table_rows.push(vec![
            server.host.clone(),
            status,
            server.index.to_string(),
            server.peer_id_index.to_string(),
            server.username.clone().unwrap_or("".to_string()).clone(),
            server.key_path.clone().unwrap_or("".to_string()).clone()
        ]
        );
    }

    ui.horizontal(|ui| {
        ui.heading("Servers");
        ui.spacing();
        ui.separator();
        ui.spacing();
        if ui.button("Deploy").clicked() {
            info!("Deploying");
            local_state.runtime.spawn(async {
                for server in servers {
                    let mut ssh = SSH::from_server(&server);
                    let is_genesis = server.index == 0;
                    let result = deploy::setup_server_redgold(
                        ssh,
                        NetworkEnvironment::Dev,
                        is_genesis,
                        None,
                        true
                    );
                    // local_state.server_state.deployment_result_info_box.lock().expect("").push(result);
                };
            });
            ()
        }
    });
    ui.separator();
    text_table(ui, table_rows);

}


pub fn app_update(app: &mut ClientApp, ctx: &egui::Context, frame: &mut eframe::Frame) {
    let ClientApp {
        logo,
        local_state,
    } = app;

    local_state.current_time = util::current_time_millis_i64();

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

    // egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
    //     // The top panel is often a good place for a menu bar:
    //     egui::menu::bar(ui, |ui| {
    //         ui.style_mut().override_text_style = Some(TextStyle::Heading);
    //         egui::menu::menu(ui, "File", |ui| {
    //             ui.style_mut().override_text_style = Some(TextStyle::Heading);
    //             if ui.button("Quit").clicked() {
    //                 frame.quit();
    //             }
    //         });
    //     });
    // });

    let img = logo;
    let texture_id = img.texture_id(ctx);

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
            ui.set_max_width(104f32);

            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::default()),
                |ui| {
                    let scale = 4;
                    let size =
                        egui::Vec2::new((img.size()[0] / scale) as f32, (img.size()[1] / scale) as f32);
                    // ui.style_mut().spacing.window_padding.y += 20.0f32;
                    ui.add_space(10f32);
                    ui.image(texture_id, size);
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
                            local_state.active_tab = tab_i;
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

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        match local_state.active_tab {
            Tab::Home => {
                home_screen(ui, ctx, local_state);
            }
            Tab::Wallet => {
                if local_state.active_passphrase.is_none() {
                    ui.heading("No wallet loaded.");
                    ui.add_space(20f32);

                    ui.heading("Enter mnemonic generation phrase:");
                    let edit = TextEdit::singleline(&mut local_state.wallet_passphrase_entry)
                        .password(!local_state.password_visible)
                        .lock_focus(true)
                        .desired_width(300f32);
                    let response = ui.add(edit);

                    if local_state.wallet_first_load_state {
                        response.request_focus();
                        local_state.wallet_first_load_state = false;
                    }
                    if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let string = local_state.wallet_passphrase_entry.clone();
                        local_state.active_passphrase = Some(string.clone());
                        local_state.visible_mnemonic = Some(
                            mnemonic_builder::from_str_rounds(&*string.clone(), 0).to_string(),
                        );
                        local_state.wallet_passphrase_entry = "".to_string();
                        //local_state.
                        // mnemonic_builder::from_str
                    }
                    if ui.button("Show password text").clicked() {
                        local_state.password_visible = !local_state.password_visible;
                    };
                } else {
                    ui.heading("Wallet loaded");

                    if ui.button("Show mnemonic").clicked() {
                        local_state.show_mnemonic = !local_state.show_mnemonic;
                    }
                    if local_state.show_mnemonic {
                        let mut option = local_state.visible_mnemonic.as_ref().unwrap().clone();
                        let edit = TextEdit::multiline(&mut option);
                        ui.add(edit);
                    }
                }
            }
            Tab::Settings => {}
            Tab::Trust => {}
            Tab::Servers => {
                servers_screen(ui, ctx, local_state);
            }
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

    // sync local data to RDS -- apart from data associated with phrases
    // discuss extra features around confirmation process. p2p negotation, contacts table.
}
