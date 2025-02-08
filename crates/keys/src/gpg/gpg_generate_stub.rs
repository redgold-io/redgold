use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::RgResult;

pub fn gpg_from_entropy(_entropy: &[u8; 32], _user_name: String, _email: Option<String>) -> RgResult<Vec<u8>> {
    "unimplemented".to_error()
}