use itertools::Itertools;
use crate::from_hex;
use crate::proto_serde::ProtoSerde;
use crate::structs::{Address, NetworkEnvironment, PeerId, PublicKey, Seed, TrustData, TrustRatingLabel};
use crate::util::current_time_millis;


impl Seed {
    pub fn port_or(&self, default: u16) -> u16 {
        self.port_offset.map(|p| p as u16).unwrap_or(default)
    }
    pub fn addresses(&self) -> Vec<Address> {
        let pid = self.peer_id.as_ref().and_then(|p| p.peer_id.as_ref());
        vec![self.public_key.as_ref(), pid].iter().flatten().flat_map(|pk| pk.address().ok()).collect_vec()
    }
}

#[derive(Debug, serde::Deserialize)]
struct SeedCsvRecord {
    external_address: String,
    network_environment: String,
    score: f64,
}


fn parse_seeds_csv_resource(str: &str) -> Vec<SeedCsvRecord> {
    let mut rdr = csv::Reader::from_reader(str.as_bytes());
    let mut res = vec![];
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: SeedCsvRecord = result.expect("");
        // println!("{:?}", record);
        res.push(record);
    }
    res
}

// TODO: Use csv parser for CLI args

pub fn seed(addr: impl Into<String>) -> Seed {
    let mut s = Seed::default();
    s.external_address = addr.into();
    let t = TrustData::from_label(0.9);
    s.trust = vec![t];
    s.environments = vec![NetworkEnvironment::All as i32];
    s
}

pub fn simple_seed(addr: impl Into<String>, pid: impl Into<String>, pk: impl Into<String>, is_main: bool) -> Seed {
    let mut ss = seed(addr);
    ss.peer_id = Some(PeerId::from_hex(pid).expect("hex"));
    ss.public_key = Some(PublicKey::from_hex(pk).expect("hex"));
    ss.environments = if is_main {
        vec![NetworkEnvironment::Main as i32]
    } else {
        NetworkEnvironment::gui_networks().iter().filter_map(|e| {
            if e.is_main() {
                None
            } else {
                Some(e.clone() as i32)
            }
        }).collect_vec()
    };
    ss
}

pub fn get_seeds() -> Vec<Seed> {
    vec![
    ]
}

pub fn get_seeds_by_env(env: &NetworkEnvironment) -> Vec<Seed> {
    get_seeds().into_iter()
        .filter(|s| {
            let env_match = s.environments.contains(&(env.clone() as i32));
            let all_env = s.environments.iter()
                .find(|e|
                    NetworkEnvironment::from_i32(**e)
                        .map(|e| e.is_all())
                        .unwrap_or(false
                        )
                ).is_some();
            let allow_all = all_env && !env.is_local_debug();
            env_match || allow_all
        })
        .collect_vec()
}

pub fn get_seeds_by_env_time(env: &NetworkEnvironment, _time: i64) -> Vec<Seed> {
    // Use this for a time match statement
    get_seeds_by_env(env)
}

// Not used, future example if necessary
fn get_seeds_csv() -> Vec<Seed> {
// Use this for triggering upgrades to the seed list
    let activation_time = 0;
    let contents = if current_time_millis() < activation_time {
        include_str!("resources/seeds/seeds-old.csv")
    } else {
        include_str!("resources/seeds/seeds.csv")
    };
    let csv_records = parse_seeds_csv_resource(contents);
    // TODO: Embed any additional data in code here afterwards.
    csv_records.iter().map(|r| {
        let mut s = Seed::default();
        s.external_address = r.external_address.clone();
        s.environments = vec![NetworkEnvironment::parse(r.network_environment.clone()) as i32];
        let mut t = TrustData::default();
        t.with_label(r.score);
        s.trust = vec![t];
        s
    }).collect_vec()
}

#[test]
fn debug() {
    // parse_seeds_csv_resource();
    // assert!(get_seeds().len() >= 3);
}