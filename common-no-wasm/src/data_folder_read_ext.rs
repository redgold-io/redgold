use redgold_schema::data_folder::EnvDataFolder;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::structs::ErrorInfo;

pub trait EnvFolderReadExt {
    async fn mnemonic(&self) -> RgResult<String>;
    async fn multiparty_import_str(&self) -> RgResult<String>;
    async fn remove_multiparty_import(&self) -> RgResult<()>;
    async fn cert(&self) -> Result<Vec<u8>, ErrorInfo>;
    async fn key(&self) -> Result<Vec<u8>, ErrorInfo>;
}

impl EnvFolderReadExt for EnvDataFolder {

    async fn mnemonic(&self) -> RgResult<String> {
        tokio::fs::read_to_string(self.mnemonic_path()).await.error_info("Bad mnemonic read")
    }

    async fn multiparty_import_str(&self) -> RgResult<String> {
        let mp_import = self.multiparty_import();
        tokio::fs::read_to_string(mp_import).await.error_info("Failed to read multiparty import")
    }

    async fn remove_multiparty_import(&self) -> RgResult<()> {
        tokio::fs::remove_file(self.multiparty_import()).await.error_info("Failed to remove multiparty import")
    }
    async fn cert(&self) -> Result<Vec<u8>, ErrorInfo> {
        tokio::fs::read(self.cert_path()).await.error_info("Missing cert")
            .or(
                tokio::fs::read(self.path.join("certificate.crt")).await
                    .error_info("Missing cert")
            )
    }

    async fn key(&self) -> Result<Vec<u8>, ErrorInfo> {
        tokio::fs::read(self.key_path()).await
            .error_info("Missing key")
            .or(
                tokio::fs::read(self.path.join("private_key.key")).await
                    .error_info("Missing key")
            )
    }


}
