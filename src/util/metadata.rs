use redgold_keys::util::mnemonic_support::WordsPassMetadata;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::helpers::easy_json::EasyJsonDeser;

pub(crate) async fn read_metadata_json(path: &String) -> RgResult<WordsPassMetadata> {
    tokio::fs::read_to_string(path).await
        .error_info("Failed to read metadata xpub file")?
        .json_from()
}