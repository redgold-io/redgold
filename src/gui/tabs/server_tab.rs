use redgold_schema::servers::Server;
use std::sync::{Arc, Mutex};
use eframe::egui::{TextEdit, Ui};
use std::path::PathBuf;
use eframe::egui;
use log::info;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use tokio::task::JoinHandle;
use crate::gui::app_loop::LocalState;
use crate::gui::common::{bounded_text_area_size_focus, editable_text_input_copy, password_single, valid_label};
use crate::gui::tables;
use crate::infra::deploy::default_deploy;
use crate::infra::{deploy, SSH};
use crate::util::cli::args::Deploy;

pub async fn update_server_status(servers: Vec<Server>, status: Arc<Mutex<Vec<ServerStatus>>>) {
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

pub fn servers_tab(ui: &mut Ui, _ctx: &egui::Context, local_state: &mut LocalState) {

    let servers = local_state.node_config.servers.clone();

    if local_state.server_state.needs_update {
        local_state.server_state.needs_update = false;
        tokio::spawn(
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
            "".to_string()
        ]
        );
    }

    ui.horizontal(|ui| {
        ui.heading("Servers");
        ui.spacing();
        ui.separator();
        ui.spacing();
    });
    ui.separator();
    tables::text_table(ui, table_rows);

    editable_text_input_copy(
        ui,"Server CSV Load Path", &mut local_state.server_state.csv_edit_path, 400.0
    );
    if ui.button("Load Servers from CSV").clicked() {
        let buf = PathBuf::from(local_state.server_state.csv_edit_path.clone());
        let res = Server::parse_from_file(buf);
        if let Ok(res) = res {
            local_state.local_stored_state.servers = res;
            local_state.persist_local_state_store();
            local_state.server_state.parse_success = Some(true);
        } else {
            local_state.server_state.parse_success = Some(false);
        }
    }
    if let Some(p) = local_state.server_state.parse_success {
        ui.horizontal(|ui| {
            ui.label("Parse result: ");
            valid_label(ui, p);
        });

    }

    ui.label("Deploy Options");

    ui.horizontal(|ui| {
        ui.checkbox(&mut local_state.server_state.words_and_id, "Words/Id");
        ui.checkbox(&mut local_state.server_state.cold, "Cold");
        ui.checkbox(&mut local_state.server_state.purge, "Purge");
        ui.checkbox(&mut local_state.server_state.ops, "Ops");
        ui.checkbox(&mut local_state.server_state.purge_ops, "Purge Ops");
        ui.checkbox(&mut local_state.server_state.skip_start, "Skip Start");
        if local_state.node_config.opts.development_mode {
            ui.checkbox(&mut local_state.server_state.genesis, "Genesis");
            ui.checkbox(&mut local_state.server_state.hard_coord_reset, "Hard Coord Reset");
        }
        ui.label("Single Server Index:");
        TextEdit::singleline(&mut local_state.server_state.server_index_edit).desired_width(50.0).show(ui);
    });

    password_single(&mut local_state.server_state.mixing_password,"Mixing Password", ui,
                    &mut local_state.server_state.show_mixing_password);

    ui.horizontal(|ui| {
        ui.checkbox(&mut local_state.server_state.load_offline_deploy, "Load Offline Deploy");
        if local_state.server_state.load_offline_deploy {
            editable_text_input_copy(ui, "Load Offline Path", &mut local_state.server_state.load_offline_path, 250.0);
        }
    });

    if ui.button("Deploy").clicked() {
        local_state.server_state.deployment_result_info_box = Arc::new(Mutex::new("".to_string()));
        info!("Deploying");
        let mut d = Deploy::default();
        if local_state.server_state.load_offline_deploy {
            d.server_offline_info = Some(local_state.server_state.load_offline_path.clone());
        }
        d.ops = local_state.server_state.ops;
        if d.ops == false {
            d.skip_ops = true;
        }
        d.purge_ops = local_state.server_state.purge_ops;
        d.debug_skip_start = local_state.server_state.skip_start;
        d.purge = local_state.server_state.purge;
        d.server_index = local_state.server_state.server_index_edit.parse::<i32>().ok();
        d.genesis = local_state.server_state.genesis;
        d.mixing_password = Some(local_state.server_state.mixing_password.clone()).filter(|s| !s.is_empty());
        d.words_and_id = local_state.server_state.words_and_id;
        d.cold = local_state.server_state.cold;

        let hard = local_state.server_state.hard_coord_reset.clone();
        if hard {
            d.hard_coord_reset = true;
            d.purge = true;
            d.debug_skip_start = true;
        }
        let config = local_state.node_config.clone();
        let mut arc = local_state.server_state.deployment_result_info_box.clone();
        let fun = Box::new(move |s: String| {
            // Lock the Mutex and get mutable access to the inner String.
            let mut inner = arc.lock().expect("lock poisoned");
            *inner = format!("{}\n{}", &*inner, s);
            info!("Deploy result: {}", s);
            Ok::<(), ErrorInfo>(())
        });
        let deploy_join = tokio::spawn(async move {
            let f = fun.clone();
            let f2 = fun.clone();

            let mut d2 = d.clone();
            let mut d3 = d2.clone();
            let nc = config.clone();
            let res = default_deploy(&mut d2, &nc, f).await;
            if hard {
                d3.debug_skip_start = false;
                let res = default_deploy(&mut d3, &nc, f2).await;
            }
            // Update final deploy result here.
        });

        local_state.server_state.deploy_process = Some(Arc::new(deploy_join));
    };

    if ui.button("Abort Deploy").clicked() {
        if let Some(join) = local_state.server_state.deploy_process.clone() {
            let j = join.clone();
            j.abort();
        }
    }

    let mut arc1 = local_state.server_state.deployment_result_info_box.clone().lock().expect("").clone();
    bounded_text_area_size_focus(ui, &mut arc1, 600., 15);

    let last_env = local_state.node_config.network.clone();

    if last_env != local_state.server_state.last_env {
        local_state.server_state.mixing_password = "".to_string();
        local_state.server_state.last_env = last_env;
    }

    ui.horizontal(|ui| {
       editable_text_input_copy(ui, "Generate Offline Path", &mut local_state.server_state.generate_offline_path, 150.0);
        if ui.button("Generate Peer TXs / Words").clicked() {
            let config1 = local_state.node_config.clone();
            tokio::spawn(deploy::offline_generate_keys_servers(
                config1,
                local_state.local_stored_state.servers.clone(),
                PathBuf::from(local_state.server_state.generate_offline_path.clone()),
                local_state.wallet_state.hot_mnemonic().words.clone(),
                local_state.wallet_state.hot_mnemonic().passphrase.clone(),
            ));
        }
    });

    if ui.button("Backup Multiparty Local Shares").clicked() {
        tokio::spawn(deploy::backup_multiparty_local_shares(
            local_state.node_config.clone(),
            local_state.local_stored_state.servers.clone(),
        ));
    }

}

#[derive(Clone)]
pub struct ServerStatus {
    pub ssh_reachable: bool
}

#[derive(Clone)]
pub struct ServersState {
    needs_update: bool,
    info: Arc<Mutex<Vec<ServerStatus>>>,
    deployment_result_info_box: Arc<Mutex<String>>,
    pub(crate) csv_edit_path: String,
    parse_success: Option<bool>,
    purge: bool,
    server_index_edit: String,
    skip_start: bool,
    pub(crate) genesis: bool,
    ops: bool,
    purge_ops: bool,
    hard_coord_reset: bool,
    words_and_id: bool,
    cold: bool,
    deployment_result: Option<String>,
    deploy_process: Option<Arc<JoinHandle<()>>>,
    mixing_password: String,
    generate_offline_path: String,
    load_offline_path: String,
    load_offline_deploy: bool,
    show_mixing_password: bool,
    last_env: NetworkEnvironment
}

impl Default for ServersState {
    fn default() -> Self {
        Self {
            needs_update: true,
            info: Arc::new(Mutex::new(vec![])),
            deployment_result_info_box: Arc::new(Mutex::new("".to_string())),
            csv_edit_path: "".to_string(),
            parse_success: None,
            purge: false,
            server_index_edit: "".to_string(),
            skip_start: false,
            genesis: false,
            ops: false,
            purge_ops: false,
            hard_coord_reset: false,
            words_and_id: false,
            cold: false,
            deployment_result: None,
            deploy_process: None,
            mixing_password: "".to_string(),
            generate_offline_path: "./servers".to_string(),
            load_offline_path: "./servers".to_string(),
            load_offline_deploy: false,
            show_mixing_password: false,
            last_env: NetworkEnvironment::Dev,
        }
    }
}