use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use itertools::Itertools;
use serde::Serialize;
use serde::Deserialize;
use crate::{error_info, ErrorInfoContext, json_or, json_pretty, RgResult};
use crate::errors::EnhanceErrorInfo;
use crate::structs::{ErrorInfo, NetworkEnvironment};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub index: i64,
    pub peer_id_index: i64,
    pub network_environment: NetworkEnvironment,
    pub username: Option<String>,
    pub ipv4: Option<String>,
    pub alias: Option<String>,
    pub external_host: Option<String>
}

fn parse_servers(str: &str) -> RgResult<Vec<Server>> {
    let mut rdr = csv::Reader::from_reader(str.as_bytes());
    let mut res = vec![];
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: Server = result.error_info("server line parse failure")?;
        res.push(record);
    }
    Ok(res)
}

impl Server {
    pub fn new(host: String) -> Self {
        Self {
            name: "".to_string(),
            host: host.clone(),
            username: None,
            ipv4: None,
            alias: None,
            index: 0,
            peer_id_index: 0,
            // TODO: Change to mainnet later
            network_environment: NetworkEnvironment::All,
            external_host: Some(host),
        }
    }

    pub fn parse_from_file(path: PathBuf)  -> Result<Vec<Self>, ErrorInfo> {
        let contents = fs::read_to_string(path).error_info("file read failure")?;
        Self::parse(contents).add("Servers file load path")
    }

    pub fn parse(contents: String) -> Result<Vec<Self>, ErrorInfo> {
        parse_servers(&contents)
    }
}

#[test]
fn parse_server_example() {
    let str = include_str!("./resources/example_servers");
    let servers = Server::parse(str.to_string()).unwrap();
    println!("{}", json_pretty(&servers).expect(""))
}