use std::path::{Path, PathBuf};
use env_logger::Env;
use redgold_schema::structs::NetworkEnvironment;

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
    pub fn peer_id_path(&self) -> PathBuf {
        self.path.join("peer_id")
    }

    pub fn servers_path(&self) -> PathBuf {
        self.path.join("servers")
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

#[derive(Clone, Debug)]
pub struct DataFolder {
    pub path: PathBuf,
}

impl DataFolder {

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