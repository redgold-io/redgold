use std::fs;
use std::str::FromStr;
use itertools::Itertools;
use serde::Serialize;
use serde::Deserialize;
use crate::{error_info, ErrorInfoContext, json_or, json_pretty};
use crate::structs::{ErrorInfo, NetworkEnvironment};

#[derive(Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub index: i64,
    pub peer_id_index: i64,
    pub network_environment: NetworkEnvironment,
    pub username: Option<String>,
    pub key_path: Option<String>,
}

impl Server {
    pub fn new(host: String) -> Self {
        Self {
            host,
            username: None,
            key_path: None,
            index: 0,
            peer_id_index: 0,
            // TODO: Change to mainnet later
            network_environment: NetworkEnvironment::All,
        }
    }

    pub fn parse_from_file(path: String)  -> Result<Vec<Self>, ErrorInfo> {
        let contents = fs::read_to_string(path).error_info("file read failure")?;
        Self::parse(contents)
    }

    pub fn parse(contents: String) -> Result<Vec<Self>, ErrorInfo> {
        let mut servers = Vec::new();
        let mut default_index = 0;
        for line in contents.lines().dropping(1) {
            let mut split = line.split(",");
            let host = split.next()
                .ok_or(error_info("missing host line in servers file"))?
                .trim().to_string();
            let mut index = default_index;
            let mut peer_id_index = 0;
            let mut network_environment = NetworkEnvironment::All;
            let mut username = None;
            let mut key_path = None;

            if let Some(x) = split.next() {
                if x.trim().len() > 0 {
                    index = i64::from_str(x.trim()).error_info(format!("invalid index: {}", x))?;
                }
            }
            if let Some(x) = split.next() {
                if x.trim().len() > 0 {
                    peer_id_index = i64::from_str(x.trim()).error_info(format!("invalid peer_id_index: {}", x))?;
                }
            }

            if let Some(x) = split.next() {
                if x.trim().len() > 0 {
                    network_environment = NetworkEnvironment::parse_safe(x.trim().to_string())?;
                }
            }
            if let Some(x) = split.next() {
                if x.trim().len() > 0 {
                    username = Some(x.trim().to_string())
                }
            }
            if let Some(x) = split.next() {
                if x.trim().len() > 0 {
                    key_path = Some(x.trim().to_string())
                }
            }

            default_index += 1;

            servers.push(Server{
                host,
                index,
                peer_id_index,
                network_environment,
                username,
                key_path,
            })

        }
        Ok(servers)
    }
}

#[test]
fn parse_server_example() {
    let str = include_str!("./resources/example_servers");
    let servers = Server::parse(str.to_string()).unwrap();
    println!("{}", json_pretty(&servers).expect(""))
}