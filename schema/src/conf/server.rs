use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use serde::Serialize;
use serde::Deserialize;
use crate::{ErrorInfoContext, RgResult};
use crate::helpers::easy_json::json_pretty;
use crate::observability::errors::EnhanceErrorInfo;
use crate::structs::{Address, ErrorInfo, NetworkEnvironment, NodeMetadata, NodeType, PeerMetadata, PublicKey, TransportInfo, VersionInfo};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Server {
    pub internal_name: String,
    pub host: String,
    pub index: i64,
    pub peer_id_index: i64,
    pub network_environment: String,
    pub username: Option<String>,
    pub ipv4: Option<String>,
    pub node_name: Option<String>,
    pub external_host: Option<String>,
    pub reward_address: Option<String>
}

impl Server {
    pub fn network_environment(&self) -> NetworkEnvironment {
        NetworkEnvironment::parse(self.network_environment.clone())
    }

    pub fn node_metadata(
        &self,
        nmd: &mut NodeMetadata
    ) -> RgResult<()> {
        let s = self;
        nmd.node_name = s.node_name.clone();
        let mut info = TransportInfo::default();
        let ti = nmd.transport_info.as_mut().unwrap_or(&mut info);
        ti.external_host = s.external_host.clone();
        ti.external_ipv4 = s.ipv4.clone();
        ti.nat_restricted = Some(false);
        nmd.transport_info = Some(ti.clone());
        Ok(())
    }

    pub fn peer_data(
        servers: Vec<Server>,
        peer_data: &mut PeerMetadata,
        peer_id_index: i64,
        pk: HashMap<i64, PublicKey>,
        checksum: String,
        net: NetworkEnvironment,
        reward_address: Option<Address>
    ) -> &mut PeerMetadata {
        let mut nmds = vec![];
        peer_data.network_environment = net.clone() as i32;
        peer_data.reward_address = reward_address;
        for s in servers {
            if s.peer_id_index == peer_id_index {
                let mut nmd = NodeMetadata::default();
                s.node_metadata(&mut nmd).expect("works");
                nmd.peer_id = peer_data.peer_id.clone();
                nmd.public_key = pk.get(&s.index).cloned();
                let mut vi = VersionInfo::default();
                vi.executable_checksum = checksum.clone();
                nmd.version_info = Some(vi);
                nmd.node_type = Some(NodeType::Static as i32);
                let option = nmd.transport_info.as_mut();
                option.expect("ti").port_offset = Some(net.default_port_offset() as i64);
                nmds.push(nmd);
            }
        }
        peer_data.node_metadata = nmds;
        peer_data
    }
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
    pub fn new(host: impl Into<String>) -> Self {
        let host_str = host.into();
        Self {
            internal_name: "".to_string(),
            host: host_str.clone(),
            username: None,
            ipv4: None,
            node_name: None,
            index: 0,
            peer_id_index: 0,
            // TODO: Change to mainnet later
            network_environment: NetworkEnvironment::All.to_std_string(),
            external_host: Some(host_str),
            reward_address: None,
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
    let str = include_str!(".././resources/example_servers");
    let servers = Server::parse(str.to_string()).unwrap();
    println!("{}", json_pretty(&servers).expect(""))
}