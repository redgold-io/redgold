use std::path::PathBuf;
use crate::helpers::easy_json::{json_from, EasyJsonDeser};
use crate::{ErrorInfoContext, RgResult};
use crate::config_data::ConfigData;
use crate::servers::ServerOldFormat;
use crate::structs::{ErrorInfo, NetworkEnvironment, Transaction};

#[derive(Clone, Debug)]
pub struct EnvDataFolder {
    pub path: PathBuf
}

impl EnvDataFolder {

    pub fn mnemonic_no_tokio(&self) -> RgResult<String> {
        std::fs::read_to_string(self.mnemonic_path()).error_info("Bad mnemonic read")
    }

    pub fn backups(&self) -> PathBuf {
        self.path.join("backups")
    }

    pub fn data_store_path(&self) -> PathBuf {
        self.path.join("data_store.sqlite")
    }

    pub fn bdk_sled_path(&self) -> PathBuf {
        self.path.join("bdk_sled")
    }

    pub fn bdk_sled_path2(&self) -> PathBuf {
        self.path.join("bdk_sled2")
    }

    pub fn mnemonic_path(&self) -> PathBuf {
        self.path.join("mnemonic")
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
    pub fn parquet_exports(&self) -> PathBuf {
        self.path.join("parquet_exports")
    }
    pub fn parquet_tx(&self) -> PathBuf {
        self.parquet_exports().join("transactions")
    }
    pub fn parquet_self_observations(&self) -> PathBuf {
        self.parquet_exports().join("observations")
    }

    pub fn servers_path(&self) -> PathBuf {
        self.path.join("servers")
    }

    pub fn servers(&self) -> RgResult<Vec<ServerOldFormat>> {
        ServerOldFormat::parse_from_file(self.servers_path())
    }

    pub fn multiparty_import(&self) -> PathBuf {
        self.path.join("multiparty-import.csv")
    }

    // Change to cert.pem
    pub fn cert_path(&self) -> PathBuf {
        //self.path.join("certificate.crt")
        PathBuf::from("/etc/letsencrypt/live/lb.redgold.io/fullchain.pem")
    }

    // Change to privkey.pem
    pub fn key_path(&self) -> PathBuf {
        // self.path.join("private_key.key")
        PathBuf::from("/etc/letsencrypt/live/lb.redgold.io/privkey.pem")
    }

    pub fn ensure_exists(&self) -> &Self {
        std::fs::create_dir_all(&self.path).ok();
        self
    }

    pub fn delete(&self) -> &Self {
        std::fs::remove_dir_all(&self.path).ok();
        self
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

    pub fn config_path(&self) -> PathBuf {
        self.path.join("config.toml")
    }

    pub fn config(&self) -> Option<RgResult<ConfigData>> {
        std::fs::read_to_string(self.config_path()).ok().map(|s| toml::from_str(&s).error_info("Bad config read"))
    }

    pub fn write_config(&self, config: &ConfigData) -> RgResult<()> {
        let string = toml::to_string(config).error_info("Bad config write")?;
        std::fs::write(self.config_path(), string).error_info("Bad config write")
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