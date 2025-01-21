use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LockScreen {
    pub iv: [u8; 16],
    pub salt: [u8; 32],
    pub hashed_pass: Option<[u8; 32]>,
    pub locked: bool,
    // This is only used by the text box and should be cleared immediately
    pub password_entry: String,
    pub encrypted_passphrase: Vec<u8>,
}