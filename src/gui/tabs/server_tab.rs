use crate::api::client::rest::RgHttpClient;
use crate::infra::deploy::default_deploy;
use crate::infra::{deploy, multiparty_backup};
use eframe::egui;
use eframe::egui::{Color32, RichText, ScrollArea, TextEdit, Ui};
use itertools::{Either, Itertools};
use redgold_common::flume_send_help::{Channel, RecvAsyncErrorInfo};
use redgold_common_no_wasm::ssh_like::DeployMachine;
use redgold_gui::common::{bounded_text_area_size_focus, editable_text_input_copy, password_single, valid_label};
use redgold_gui::components::tables;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_gui::tab::deploy::deploy_state::{ServerStatus, ServersState};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::Deploy;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use redgold_schema::RgResult;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};
use redgold_gui::state::local_state::LocalState;

pub trait ServerClient {
    fn client(&self, network_environment: &NetworkEnvironment) -> RgHttpClient;
}

impl ServerClient for ServerOldFormat {
    fn client(&self, network_environment: &NetworkEnvironment) -> RgHttpClient {
        let h = if self.host.is_empty() {
            "127.0.0.1".to_string()
        } else {
            self.host.clone()
        };
        RgHttpClient::from_env(h, network_environment)
    }
}

pub async fn update_server_status(
    servers: Vec<ServerOldFormat>, status: Arc<Mutex<Vec<ServerStatus>>>,
    network_environment: NetworkEnvironment
) {

    for server in servers {
        if server.host.is_empty() {
            continue;
        }
        let mut ssh = DeployMachine::new(&server, None, None);
        let reachable = ssh.verify().await.is_ok();
        let docker_ps_online = ssh.verify_docker_running(&network_environment).await.is_ok();
        let client = server.client(&network_environment);
        let metrics = client.metrics_map().await.ok();
        let tables = client.table_sizes_map().await.ok();
        let this_status = ServerStatus{
            server_index: server.index,
            ssh_reachable: reachable,
            docker_ps_online,
            tables,
            metrics,
        };
        {
            let mut guard = status.lock().expect("lock");
            guard.retain(|x| x.server_index != server.index);
            guard.push(this_status);
        }
    };

}

pub fn servers_tab<G>(
    ui: &mut Ui,
    _ctx: &egui::Context,
    state: &mut ServersState,
    g: &G,
    nc: &NodeConfig,
    words: String,
    passphrase: Option<String>
)
where G: GuiDepends + Clone + Send + 'static {
    let config_data = g.get_config();
    let servers = config_data.servers_old();

    if state.needs_update {
        state.needs_update = false;
        g.spawn(
            update_server_status(
                servers.clone(),
                state.info.clone(),
                g.get_network().clone()
            )
        );
    };
    let info = state.info.lock().expect("").to_vec();

    let mut table_rows: Vec<Vec<String>> = vec![];
    table_rows.push(vec![
        "Hostname".to_string(),
        "Index".to_string(),
        "Peer".to_string(),
        "SSH".to_string(),
        "Docker".to_string(),
        "TX Total".to_string(),
        // "SSH Key Path".to_string(),
    ]);

    for (i, server) in servers.iter().enumerate() {
        let status_i = info.get(i);
        let status = status_i.map(|s| match s.ssh_reachable {
            true => {"Online"}
            false => {"Offline"}
        }).unwrap_or("querying").to_string();
        let tx_total = status_i.map(|s| match s.metrics.as_ref()
            .and_then(|m| m.get("redgold_transaction_accepted_total")) {
            Some(t) => {t.as_str()}
            None => {"failure"}
        }).unwrap_or("querying").to_string();
        let process_up = status_i.map(|s| match s.ssh_reachable {
            true => {"Online"}
            false => {"Offline"}
        }).unwrap_or("querying").to_string();

        table_rows.push(vec![
            server.host.clone(),
            server.index.to_string(),
            server.peer_id_index.to_string(),
            status,
            process_up,
            tx_total
            // server.username.clone().unwrap_or("".to_string()).clone(),
            // "".to_string()
        ]
        );
    }

    ui.horizontal(|ui| {
        ui.heading("Servers");
        ui.spacing();
        ui.separator();
        ui.spacing();
        if ui.button("Refresh").clicked() {
            state.needs_update = true;
        }
    });
    ui.separator();

    ui.horizontal(|ui| {

    ScrollArea::vertical().id_source("tabletext")
        .max_height(150.0)
        .min_scrolled_height(150.0)
        .max_width(600.0)
        .min_scrolled_width(600.0)
        .auto_shrink(true)
        .show(ui, |ui| {
            ui.vertical(|ui| {
            tables::text_table(ui, table_rows);
            });
        });

    });
    // 
    // ui.horizontal(|ui| {
    //     editable_text_input_copy(
    //         ui,"Server CSV Load Path", &mut state.csv_edit_path, 300.0
    //     );
    //     if ui.button("Load").clicked() {
    //         let buf = PathBuf::from(state.csv_edit_path.clone());
    //         let res = ServerOldFormat::parse_from_file(buf);
    //         if let Ok(res) = res {
    //             local_state.local_stored_state.servers = Some(res);
    //             local_state.persist_local_state_store();
    //             state.parse_success = Some(true);
    //         } else {
    //             state.parse_success = Some(false);
    //         }
    //     }
    // });
    // if let Some(p) = state.parse_success {
    //     ui.horizontal(|ui| {
    //         ui.label("Parse result: ");
    //         valid_label(ui, p, );
    //     });
    // 
    // }

    // TODO: Move all this to unification with the deploy args
    ui.label("Deploy Options");

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.redgold_process, "Redgold Process");
        ui.checkbox(&mut state.words_and_id, "Words/Id");
        ui.checkbox(&mut state.cold, "Cold");
        ui.checkbox(&mut state.purge, "Purge");
        ui.label("Server Filter:");
        TextEdit::singleline(&mut state.server_index_edit).desired_width(50.0).show(ui);
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.ops, "Ops");
        ui.checkbox(&mut state.system, "Apt/System");
        ui.checkbox(&mut state.purge_ops, "Purge Ops");
        ui.checkbox(&mut state.skip_logs, "Skip Logging");
    });

    if config_data.development_mode() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut state.skip_start, "Skip Start");
            ui.checkbox(&mut state.genesis, "Genesis");
            ui.checkbox(&mut state.hard_coord_reset, "Hard Coord Reset");
        });
    }

    password_single(&mut state.mixing_password,"Mixing Password", ui,
                    &mut state.show_mixing_password);

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.load_offline_deploy, "Load Offline Deploy");
        if state.load_offline_deploy {
            editable_text_input_copy(ui, "Load Offline Path", &mut state.load_offline_path, 250.0);
        }
    });
    ui.horizontal(|ui| {
    if ui.button("Deploy").clicked() {
        state.deployment_result_info_box = Arc::new(Mutex::new("".to_string()));
        state.deployment_result = Arc::new(Mutex::new(Either::Left(None)));
        info!("Deploying");
        let mut d = Deploy::default();
        if state.load_offline_deploy {
            d.server_offline_info = Some(state.load_offline_path.clone());
        }
        d.ops = state.ops;
        if d.ops == false {
            d.skip_ops = true;
        }
        d.disable_apt_system_init = !state.system;
        d.skip_redgold_process = !state.redgold_process;
        d.skip_logs = state.skip_logs;
        d.purge_ops = state.purge_ops;
        d.debug_skip_start = state.skip_start;
        d.purge = state.purge;
        d.server_index = state.server_index_edit.parse::<i32>().ok();
        d.server_filter = Some(state.server_index_edit.clone());
        d.genesis = state.genesis;
        d.mixing_password = Some(state.mixing_password.clone()).filter(|s| !s.is_empty());
        d.words_and_id = state.words_and_id;
        d.cold = state.cold;

        let hard = state.hard_coord_reset.clone();
        if hard {
            d.hard_coord_reset = true;
            // d.purge = true;
            d.debug_skip_start = true;
        }
        let config = nc.clone();
        let arc = state.deployment_result_info_box.clone();

        let c: Channel::<String> = Channel::new();
        let r = c.receiver.clone();
        let default_fun = g.spawn(async move {
            loop {
                let s = match r.recv_async_err().await {
                    Ok(x) => {
                        x
                    }
                    Err(e) => {
                        error!("Channel receive error: {}", e.json_or());
                        break;
                    }
                };
                let mut inner = arc.lock().expect("lock poisoned");
                let s = s.trim();
                if s.is_empty() {
                    continue;
                }
                *inner = format!("{}\n{}", &*inner, s);
                info!("Deploy result: {}", s);
            }
            ()
        });

        let deployment = config_data.local
            .and_then(|x| x.deploy.clone())
            .map(|d| d.fill_params());

        let deploy_interrupt: Channel::<()> = Channel::new();
        state.interrupt_sender = Some(deploy_interrupt.sender.clone());


        let output_handler = Some(c.sender.clone());
        let arc = state.deployment_result.clone();
        let s2 = servers.clone();
        let deploy_join = g.spawn_interrupt(async move {
            let f = output_handler.clone();
            let f2 = output_handler.clone();
            let mut d2 = d.clone();
            let mut d3 = d2.clone();
            let nc = config.clone();
            let s = s2.clone();
            let dpl = deployment.clone();
            let _res = default_deploy(&mut d2, &nc, f, Some(s), dpl.clone()).await;
            info!("Deploy complete {}", _res.json_or());
            *arc.lock().expect("") = Either::Left(Some(_res));
            if hard {
                d3.debug_skip_start = false;
                let _res = default_deploy(&mut d3, &nc, f2, None, dpl).await;
            }
            // default_fun.abort();
            // Update final deploy result here.
        }, deploy_interrupt.receiver.clone());

        // state.deploy_process = Some(Arc::new(deploy_join));
    };

    match state.deployment_result.lock().expect("").as_ref() {
        Either::Left(l) => {
            match l {
                None => {
                    ui.label(RichText::new("Running").color(Color32::WHITE));
                }
                Some(Ok(_)) => {
                    ui.label(RichText::new("Success").color(Color32::GREEN));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new("Deployment Error").color(Color32::RED));
                }
            }
        }
        Either::Right(_) => {
            ui.label(RichText::new("Click to Deploy").color(Color32::WHITE));
        }
    }


    if ui.button("Abort Deploy").clicked() {
        if let Some(join) = state.interrupt_sender.clone() {
            join.send(());
        }
    }
    });

    let mut arc1 = state.deployment_result_info_box.clone().lock().expect("").clone();
    bounded_text_area_size_focus(ui, &mut arc1, 500., 10);

    let last_env = g.get_network().clone();

    if last_env != state.last_env {
        state.mixing_password = "".to_string();
        state.last_env = last_env;
    }

    ui.horizontal(|ui| {
        editable_text_input_copy(ui, "Generate Offline Path", &mut state.generate_offline_path, 150.0);
        editable_text_input_copy(ui, "Pass", &mut state.generate_offline_pass, 50.0);
        // editable_text_input_copy(ui, "Offset", &mut state.generate_offline_offset, 50.0);
        if ui.button("Generate Peer TXs / Words").clicked() {
            // let offset = state.generate_offline_offset.parse::<u32>().unwrap_or(0);
            // let w = G::hash_derive_words()
            let config1 = nc.clone();
            let option = servers.clone();
            let string = state.generate_offline_words.clone();
            let passoptstr = state.generate_offline_pass.clone();
            let p = if passoptstr.is_empty() {
                None
            } else {
                Some(passoptstr)
            };
            tokio::spawn(deploy::offline_generate_keys_servers(
                config1,
                option,
                PathBuf::from(state.generate_offline_path.clone()),
                string,
                p,
            ));
        }
    });
    editable_text_input_copy(ui, "Words", &mut state.generate_offline_words, 300.0);

    ui.horizontal(|ui| {
        if ui.button("Backup Multiparty Local Shares").clicked() {
            let option1 = servers.clone();
            tokio::spawn(multiparty_backup::backup_multiparty_local_shares(
                nc.clone(),
                option1,
            ));
        }

        if ui.button("Backup Datastores").clicked() {
            // let option1 = servers.clone();
            g.backup_data_stores();
        };

        if ui.button("Restore Datastores").clicked() {
            // let option1 = servers.clone();
            // state.server_index_edit
            g.restore_data_stores(None);
        };

    });

}

