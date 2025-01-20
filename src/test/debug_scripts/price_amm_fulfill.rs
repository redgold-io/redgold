use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::SupportedCurrency;
use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};

#[ignore]
#[tokio::test]
async fn debug() {
    let nc = NodeConfig::dev_default().await;
    let party_data = nc.api_rg_client().party_data().await.log_error().unwrap();
    let p = party_data.into_iter().next().unwrap().1;
    let p = p.party_events.unwrap().central_prices.get(&SupportedCurrency::Ethereum).cloned().unwrap();
    let amt = (0.04143206 * 1e8) as u64;
    let result = p.dummy_fulfill(amt, false, &nc.network, SupportedCurrency::Ethereum);
    println!("Result: {:?}", result);
}


//
// fn encrypt(&self, str: String) -> Vec<u8> {
//     return sym_crypt::encrypt(
//         str.as_bytes(),
//         &self.session_password_hashed.unwrap(),
//         &self.iv,
//     )
//     .unwrap();
// }
//
// fn decrypt(&self, data: &[u8]) -> Vec<u8> {
//     return sym_crypt::decrypt(data, &self.session_password_hashed.unwrap(), &self.iv).unwrap();
// }
//
// fn accept_passphrase(&mut self, pass: String) {
//     let encrypted = self.encrypt(pass);
//     self.stored_passphrase = encrypted;
// } // https://www.quora.com/Is-it-useful-to-multi-hash-like-10-000-times-a-password-for-an-anti-brute-force-encryption-algorithm-Do-different-challenges-exist
//
// fn hash_password(&mut self) -> [u8; 32] {
//     let mut vec = self.password_entry.as_bytes().to_vec();
//     vec.extend(self.session_salt.to_vec());
//     return dhash_vec(&vec);
// }
// fn store_password(&mut self) {
//     self.session_password_hashed = Some(self.hash_password());
// }