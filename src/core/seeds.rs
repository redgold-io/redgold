use bitcoin::secp256k1::PublicKey;
use redgold_schema::structs::{NetworkEnvironment, TrustData};

#[derive(Clone, Debug)]
pub struct SeedNode {
    pub peer_id: Option<Vec<u8>>,
    pub trust: Vec<TrustData>,
    pub public_key: Option<PublicKey>,
    pub external_address: String,
    pub port_offset: Option<u16>,
    pub environments: Vec<NetworkEnvironment>,
}
//
// impl SeedNode {
//     fn from_environment(network_environment: NetworkEnvironment) {
//         match network_environment {
//             _ => {
//                 vec![
//                     SeedNode{
//                         peer_id: vec![],
//                         trust: 0.0,
//                         public_key: None,
//                         external_address: "".to_string(),
//                         port: 0,
//                     }
//                 ]
//             }
//             // NetworkEnvironment::Main => {}
//             // NetworkEnvironment::Test => {}
//             // NetworkEnvironment::Dev => {}
//             // NetworkEnvironment::Staging => {}
//             // NetworkEnvironment::Perf => {}
//             // NetworkEnvironment::Integration => {}
//             // NetworkEnvironment::Local => {}
//             // NetworkEnvironment::Debug => {}
//             // NetworkEnvironment::All => {}
//             // NetworkEnvironment::Predev => {}
//         }
//     }
// }
