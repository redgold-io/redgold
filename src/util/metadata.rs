use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::keys::words_pass::WordsPassMetadata;
pub(crate) async fn read_metadata_json(path: &String) -> RgResult<WordsPassMetadata> {
    tokio::fs::read_to_string(path).await
        .error_info("Failed to read metadata xpub file")?
        .json_from()
}