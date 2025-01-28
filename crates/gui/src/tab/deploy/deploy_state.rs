use either::Either;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::RgResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ServerStatus {
    pub server_index: i64,
    pub ssh_reachable: bool,
    pub docker_ps_online: bool,
    pub tables: Option<HashMap<String, i64>>,
    pub metrics: Option<HashMap<String, String>>
}

#[derive(Clone)]
pub struct ServersState {
    pub needs_update: bool,
    pub info: Arc<Mutex<Vec<ServerStatus>>>,
    pub deployment_result_info_box: Arc<Mutex<String>>,
    parse_success: Option<bool>,
    pub purge: bool,
    pub server_index_edit: String,
    pub skip_start: bool,
    pub genesis: bool,
    pub ops: bool,
    pub redgold_process: bool,
    pub skip_logs: bool,
    pub purge_ops: bool,
    pub hard_coord_reset: bool,
    pub words_and_id: bool,
    pub cold: bool,
    pub deployment_result: Arc<Mutex<Either<Option<RgResult<()>>, ()>>>,
    pub interrupt_sender: Option<flume::Sender<()>>,
    pub mixing_password: String,
    pub generate_offline_path: String,
    pub load_offline_path: String,
    pub load_offline_deploy: bool,
    pub show_mixing_password: bool,
    pub last_env: NetworkEnvironment,
    pub system: bool
}

impl Default for ServersState {
    fn default() -> Self {
        Self {
            needs_update: true,
            info: Arc::new(Mutex::new(vec![])),
            deployment_result_info_box: Arc::new(Mutex::new("".to_string())),
            parse_success: None,
            purge: false,
            server_index_edit: "".to_string(),
            skip_start: false,
            genesis: false,
            ops: true,
            redgold_process: true,
            skip_logs: false,
            purge_ops: false,
            hard_coord_reset: false,
            words_and_id: true,
            cold: false,
            deployment_result: Arc::new(Mutex::new(Either::Right(()))),
            interrupt_sender: None,
            mixing_password: "".to_string(),
            generate_offline_path: "./servers".to_string(),
            load_offline_path: "./servers".to_string(),
            load_offline_deploy: false,
            show_mixing_password: false,
            last_env: NetworkEnvironment::Dev,
            system: true,
        }
    }
}