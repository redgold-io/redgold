use crate::{error_info, HashClear, RgResult, SafeOption};
use crate::structs::{ErrorInfo, NetworkEnvironment, NodeMetadata, PeerNodeInfo, PublicKey, TransportInfo};

impl HashClear for NodeMetadata {
    fn hash_clear(&mut self) {}
}

impl NodeMetadata {
    pub fn port_or(&self, network: NetworkEnvironment) -> u16 {
        self.transport_info.as_ref()
            .and_then(|ti| ti.port_offset)
            .unwrap_or(network.default_port_offset() as i64) as u16
    }
    pub fn public_key_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        let x = self.public_key.safe_get()?;
        let y= x.bytes.safe_get()?.value.clone();
        Ok(y)
    }

    pub fn long_identifier(&self) -> String {
        let pk = self.public_key.clone().and_then(|p| p.hex().ok()).unwrap_or("".to_string());
        let info = self.transport_info.clone().expect("ti");
        let ip = info.external_host.or(info.external_ipv4).expect("ip");
        format!("node_name {} public key {} ip {}",
                self.node_name.clone().unwrap_or("none".to_string()), pk, ip,
        )
    }

    pub fn external_address(&self) -> RgResult<String> {
        self.transport_info.safe_get_msg("Missing transport info")
            .and_then(|t| t.external_address())
    }

}


impl PeerNodeInfo {
    pub fn public_keys(&self) -> Vec<PublicKey> {
        let mut res = vec![];
        if let Some(t) = &self.latest_node_transaction {
            if let Ok(p) = &t.peer_data() {
                for n in &p.node_metadata {
                    if let Some(p) = &n.public_key {
                        res.push(p.clone());
                    }
                }
            }
        }

        if let Some(t) = &self.latest_node_transaction {
            if let Ok(n) = &t.node_metadata() {
                if let Some(p) = &n.public_key {
                    res.push(p.clone());
                }
            }
        }

        res
    }
}

impl TransportInfo {
    fn external_address(&self) -> RgResult<String> {
        self.external_host.as_ref().or(self.external_ipv4.as_ref()).safe_get_msg("No external address")
            .cloned()
            .cloned()
    }
}