use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::RgResult;

pub fn gpg_from_entropy(entropy: &[u8; 32], user_name: String, email: Option<String>) -> RgResult<Vec<u8>> {
    "unimplemented".to_error()
}