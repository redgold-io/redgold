use crate::proto_serde::ProtoSerde;
use crate::structs::{ErrorInfo, Hash, NetworkEnvironment, NodeMetadata, PartitionInfo, PeerNodeInfo, PublicKey, TransportInfo};
use crate::util::xor_distance::xorfc_hash;
use crate::{HashClear, RgResult, SafeOption};

impl HashClear for NodeMetadata {
    fn hash_clear(&mut self) {}
}

impl NodeMetadata {

    pub fn nat_traversal_required(&self) -> bool {
        self.transport_info
            .as_ref()
            .and_then(|t| t.nat_restricted)
            .unwrap_or(false)
    }
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
        let pk = self.public_key.clone().map(|p| p.hex()).unwrap_or("".to_string());
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

    pub fn tx_in_range(&self, h: &Hash) -> bool {
        self.hash_in_range(h, |i| i.transaction_hash)
    }

    pub fn hash_in_range(&self, h: &Hash, f: fn(&PartitionInfo) -> Option<i64>) -> bool {
        if let Some(d) = self.partition_info.as_ref().and_then(f) {
            let pk = self.public_key.as_ref().expect("pk");
            xorfc_hash(h, pk) < d
        } else {
            true
        }
    }

}


impl PeerNodeInfo {

    pub fn nmd_pk(&self) -> Option<PublicKey> {
        if let Some(t) = &self.latest_node_transaction {
            if let Ok(n) = t.node_metadata() {
                if let Some(p) = n.public_key {
                    return Some(p);
                }
            }
        }
        None
    }
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

        if let Some(pk) = self.nmd_pk() {
            res.push(pk.clone());
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