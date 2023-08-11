use crate::{ErrorInfoContext, SafeBytesAccess};
// TODO: Should we just switch to truncated SHA512?

// https://bitcoin.stackexchange.com/questions/8443/where-is-double-hashing-performed-in-bitcoin
/*
To avoid this property, Ferguson and Schneier suggested using SHA256d = SHA256(SHA256(x))
 which avoids length-extension attacks. This construction has some minor weaknesses
  (not relevant to bitcoin), so I wouldn't recommend it for new protocols,
  and would use HMAC with constant key, or truncated SHA512 instead.
 */

pub mod merkle;
pub mod xor_distance;
pub mod lang_util;

pub fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub fn make_ascii_titlecase(s: &mut str) -> String {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    return s.to_string();
}
