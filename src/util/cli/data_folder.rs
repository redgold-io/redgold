use std::path::{Path, PathBuf};
use redgold_schema::{ErrorInfoContext, json_from, RgResult, structs};
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, Transaction};
use redgold_data::data_store::DataStore;
use redgold_schema::servers::Server;

// TODO: Move everything to use this

#[derive(Clone, Debug)]
pub struct EnvDataFolder {
    pub path: PathBuf
}

impl EnvDataFolder {

    pub fn data_store_path(&self) -> PathBuf {
        self.path.join("data_store.sqlite")
    }

    pub fn mnemonic_path(&self) -> PathBuf {
        self.path.join("mnemonic")
    }

    pub async fn mnemonic(&self) -> RgResult<String> {
        tokio::fs::read_to_string(self.mnemonic_path()).await.error_info("Bad mnemonic read")
    }

    pub fn peer_tx(&self) -> RgResult<Transaction> {
        let contents = std::fs::read_to_string(self.peer_tx_path()).error_info("Bad peer tx read")?;
        json_from(&*contents)
    }

    pub fn peer_id_path(&self) -> PathBuf {
        self.path.join("peer_id")
    }

    pub fn peer_tx_path(&self) -> PathBuf {
        self.path.join("peer_tx")
    }

    pub fn metrics_list(&self) -> PathBuf {
        self.path.join("metrics_list")
    }

    pub fn targets(&self) -> PathBuf {
        self.path.join("targets.json")
    }

    pub fn servers_path(&self) -> PathBuf {
        self.path.join("servers")
    }

    pub fn servers(&self) -> RgResult<Vec<Server>> {
        Server::parse_from_file(self.servers_path())
    }

    // Change to cert.pem
    pub fn cert_path(&self) -> PathBuf {
        self.path.join("certificate.crt")
    }

    pub async fn cert(&self) -> Result<Vec<u8>, ErrorInfo> {
        tokio::fs::read(self.cert_path()).await.error_info("Missing cert")
    }

    pub async fn key(&self) -> Result<Vec<u8>, ErrorInfo> {
        tokio::fs::read(self.key_path()).await.error_info("Missing key")
    }

    // Change to privkey.pem
    pub fn key_path(&self) -> PathBuf {
        self.path.join("private_key.key")
    }

    pub fn ensure_exists(&self) -> &Self {
        std::fs::create_dir_all(&self.path).ok();
        self
    }

    pub fn delete(&self) -> &Self {
        std::fs::remove_dir_all(&self.path).ok();
        self
    }

    pub async fn data_store(&self) -> DataStore {
        // TODO: From file path
        DataStore::from_file_path(self.data_store_path().to_str().expect("Data store path").to_string()).await
    }

}

#[derive(Clone, Debug)]
pub struct DataFolder {
    pub path: PathBuf,
}

impl DataFolder {

    pub fn from_string(path: String) -> Self {
        Self{path: PathBuf::from(path)}
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self{path}
    }

    pub fn all(&self) -> EnvDataFolder {
        self.by_env(NetworkEnvironment::All)
    }

    pub fn by_env(&self, env: NetworkEnvironment) -> EnvDataFolder {
        let path = self.path.join(env.to_std_string());
        let ret = EnvDataFolder { path };
        // TODO: Remove this
        ret.ensure_exists();
        ret
    }

    pub fn target(id: u32) -> Self {
        let cwd = std::env::current_dir().expect("Current dir");
        let cwd_target = cwd.join("target");
        Self{path: cwd_target.join(format!("node_{}", id))}
    }

    pub fn ensure_exists(&self) -> &Self {
        std::fs::create_dir_all(&self.path).expect("Failed to create data folder");
        self
    }

    pub fn delete(&self) -> &Self {
        std::fs::remove_dir_all(&self.path).ok();
        self
    }

}

#[test]
fn debug() {

}