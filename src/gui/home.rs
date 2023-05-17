use std::sync::Arc;
use crossbeam::atomic::AtomicCell;
use eframe::egui::Ui;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use std::collections::HashMap;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use itertools::Itertools;
use std::time::Duration;
use log::{error, info};
use crate::gui::app_loop;
use crate::gui::app_loop::LocalState;
use crate::gui::tables::text_table;
use crate::node_config::NodeConfig;
use crate::util;

pub fn gui_status_networks() -> Vec<NetworkEnvironment> {
    // let mut vec = NetworkEnvironment::status_networks();
    vec![NetworkEnvironment::Dev]
    // vec
}

pub struct HomeState {
    pub network_status_info: Arc<AtomicCell<Vec<NetworkStatusInfo>>>,
    pub last_query_started_time: Option<i64>
}

impl HomeState {
    pub fn from() -> Self {
        Self {
            network_status_info: Arc::new(AtomicCell::new(vec![])),
            last_query_started_time: None,
        }
    }
}

pub fn home_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    ui.heading("Home");
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

    let mut table_data: Vec<Vec<String>> = vec![];
    let headers = vec!["Network".to_string(), "Status".to_string()];
    table_data.push(headers.clone());


    // let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
    // let networks = gui_status_networks();
    // let num_rows = networks.len();
    // let mut network_index = HashMap::new();
    // let row_network: Vec<String> = networks
    //     .iter().enumerate().map(|(index, x)| {
    //     network_index.insert(index, x.clone());
    //     x.to_std_string()
    // }).collect_vec();

    // Well this is ridiculous
    // can we change from atomic cell or use some copyable type?
    let status_info = home_state.network_status_info.take();
    home_state.network_status_info.store(status_info.clone());

    if status_info.is_empty() {
        let networks = gui_status_networks();
        let rows = networks.iter().map(|n| {
            let mut v = vec![];
            v.push(n.to_std_string());
            let num_fill_rows = headers.len() - 1;
            for _ in 0..num_fill_rows {
                v.push("querying".to_string());
            }
            v
        }).collect_vec();
        table_data.extend(rows);
    }

    for s in status_info.clone() {
        let vec1 = vec![
            s.network.to_std_string(),
            match s.reachable {
                true => { "Online" }
                false => { "Offline" }
            }.to_string()
        ];
        table_data.push(vec1);
    }
    text_table(ui, table_data);

}

#[derive(Clone)]
pub struct NetworkStatusInfo{
    pub network_index: usize,
    pub network: NetworkEnvironment,
    pub reachable: bool,
    // num_peers: usize,
    // num_transactions: usize,
    // genesis_hash_short: String,
    pub s3_release_exe_hash: String,
    pub peers: i64,
    pub total_tx: i64,
}

pub async fn query_network_status(
    node_config: NodeConfig,
    result: Arc<AtomicCell<Vec<NetworkStatusInfo>>>
) -> Result<(), ErrorInfo> {

    let mut results = vec![];
    for (i, x) in gui_status_networks().iter().enumerate() {
        let s3_release_exe_hash = util::auto_update::get_s3_sha256_release_hash(x.clone(), None)
            .await.unwrap_or("".to_string());
        // info!("Release exe hash: {}", release_exe_hash);
        let mut config = node_config.clone();
        config.network = x.clone();
        let mut client = config.lb_client();
        client.timeout = Duration::from_secs(2);
        let res = client.about().await;
        let mut peers = 0;
        let mut total_tx = 0;

        let reachable = match res {
            Ok(a) => {
                peers = a.num_active_peers;
                total_tx = a.total_accepted_transactions;
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
            s3_release_exe_hash,
            peers,
            total_tx
        };
        results.push(status);
    }
    result.store(results.clone());
    let map2 = result.take();
    result.store(map2.clone());
    // info!("Network status: {}", map2.to_string());
    Ok(())
}
