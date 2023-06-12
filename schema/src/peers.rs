use crate::{HashClear, SafeOption};
use crate::structs::{ErrorInfo, NetworkEnvironment, NodeMetadata};

impl HashClear for NodeMetadata {
    fn hash_clear(&mut self) {}
}

impl NodeMetadata {
    pub fn port_or(&self, network: NetworkEnvironment) -> u16 {
        self.port_offset.unwrap_or(network.default_port_offset() as i64) as u16
    }
    pub fn public_key_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        let x = self.public_key.safe_get()?;
        let y= x.bytes.safe_get()?.value.clone();
        Ok(y)
    }

    pub fn long_identifier(&self) -> String {
        let pk = self.public_key.clone().and_then(|p| p.hex().ok()).unwrap_or("".to_string());
        let ip = self.external_address.clone();
        let environment = NetworkEnvironment::from_i32(self.network_environment).unwrap_or(NetworkEnvironment::Debug);
        format!("alias {} public key {} ip {}:{} network: {}",
                self.alias.clone().unwrap_or("none".to_string()), pk, ip,
                self.port_or(environment),
                (environment).to_std_string()
        )
    }

}
