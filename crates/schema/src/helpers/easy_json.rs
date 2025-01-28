use crate::observability::errors::EnhanceErrorInfo;
use crate::structs::{ErrorInfo, VersionInfo};
use crate::{ErrorInfoContext, RgResult};
use serde::Serialize;

// #[async_trait]
pub trait EasyJson {
    fn json(&self) -> anyhow::Result<String, ErrorInfo>;
    fn json_or(&self) -> String;
    fn json_pretty(&self) -> anyhow::Result<String, ErrorInfo>;
    fn json_pretty_or(&self) -> String;
    fn write_json(&self, path: &str) -> RgResult<()>;
}

pub trait EasyJsonDeser {
    fn json_from<'a, T: serde::Deserialize<'a>>(&'a self) -> anyhow::Result<T, ErrorInfo>;
}

impl EasyJsonDeser for String {
    fn json_from<'a, T: serde::Deserialize<'a>>(&'a self) -> anyhow::Result<T, ErrorInfo> {
        json_from(self)
    }
}

// #[async_trait]
impl<T> EasyJson for T
where T: Serialize {
    fn json(&self) -> anyhow::Result<String, ErrorInfo> {
        json(&self)
    }

    fn json_or(&self) -> String {
        json_or(&self)
    }

    fn json_pretty(&self) -> anyhow::Result<String, ErrorInfo> {
        json_pretty(&self)
    }
    fn json_pretty_or(&self) -> String {
        json_pretty(&self).unwrap_or("json pretty failure".to_string())
    }

    fn write_json(&self, path: &str) -> RgResult<()> {
        let string = self.json_or();
        std::fs::write(path, string.clone()).error_info("error write json to path ").add(path.to_string()).add(" ").add(string)
    }

}

#[test]
pub fn json_trait_ser_test() {
    let mut vers = VersionInfo::default();
    vers.executable_checksum = "asdf".to_string();
    println!("{}", vers.json_or());
}

pub fn json<T: Serialize>(t: &T) -> anyhow::Result<String, ErrorInfo> {
    serde_json::to_string(&t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {:?}", e)))
}

pub fn json_result<T: Serialize, E: Serialize>(t: &anyhow::Result<T, E>) -> String {
    match t {
        Ok(t) => json_or(t),
        Err(e) => json_or(e),
    }
}

pub fn json_or<T: Serialize>(t: &T) -> String {
    json(t).unwrap_or("json ser failure of error".to_string())
}

pub fn json_pretty<T: Serialize>(t: &T) -> anyhow::Result<String, ErrorInfo> {
    serde_json::to_string_pretty(&t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {:?}", e)))
}

pub fn json_from<'a, T: serde::Deserialize<'a>>(t: &'a str) -> anyhow::Result<T, ErrorInfo> {
    serde_json::from_str(t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {:?}", e)))
}
