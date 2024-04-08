use itertools::Itertools;
use crate::from_hex;
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
        // Mainnet
        simple_seed("n0.redgold.io", "03d2ae18060412ca705243f414df883511fa5186eec7dc94091efb605569cdfd3f",
                    "030b61dae813aca2588a5e0e7e3b3dd60cc6f12487a3d7326efedeaeade6bf0d77", true),
        simple_seed("n1.redgold.io", "02935aad7a0fb43eef3c5d662c80c412acd16e8203814dcbbe5a39616cfd007abb",
                    "02df70737b5e4a42ad5b623f41b7357494c54fddab864623a47880737b5c524335", true),
        simple_seed("n3.redgold.io", "02b4a7cf97fb1d9d9b0e6e25537ea0c23028e6237ded64506070c7f69687fdfde1",
                    "021ba3977368bac89015626a2c1107ab37de9feea00c6158a90ba6ae838ef17f3b", true),
        simple_seed("n4.redgold.io", "03aaff4196db42421cd6fd8743ea83d6601a5e5fb75dc3927723688449344a756d",
        "023b7bc0577bd66e0a29dfe2c3c826de8b2c55701e0ed5a5dd24960b270c6888aa", true),

        // Testnet / dev / staging

        simple_seed(
            "n0.redgold.io",
            "028f78d7ac63a35087092f05981a115bf42b6afc31fd97eb4107b1e2b73d27f98a",
            "03a59d904435a72b9d97bddece79692bef51a0fb030b277deb9dc69b75ebc38c6f",
            false),
        simple_seed(
            "n1.redgold.io",
            "02e81515dfc521dbfa5ed4ea952f8d080612f1782063f97fe5bbe858c470710027",
            "03219cb61b1463c1c507501f6621d18092cf44ee9c49c8e998a56c638da5b03b07",
            false),
        simple_seed(
            "n2.redgold.io",
            "03dc0949cb7027639a8500b43986e34fd5eff774d968cb84133160b3b330c259aa",
            "03413bfd6768962e291e57047459baff1be8869e185b75c06c99fde66899aeb787",
            false),
        simple_seed(
            "n3.redgold.io",
            "022ac18c9106276a154abe929b1bb0f3b7004db924d62a2d37fc9c2a3f4f996c28",
            "0274a5a0382173ab16706782caaa9c9b4c2bca8dcd5d6321460c8e5619be830913",
            false),
        simple_seed(
            "n4.redgold.io",
            "0322841b5833ddb734ecec64ffd03250141a22d34978626d8db670f5eb21fef14f",
            "03bac75d3501cfe5893324cc79bbfb91f471712547dc30b26b34f7fedcb05ce1fe",
            false),
        simple_seed(
            "n5.redgold.io",
            "023e4471aa6ad46d04e6e92642b5d6b52ef968022568ee39241cdb6daea0c888a0",
            "022a557a6f3b27a5b734665231d9eca9b38cdb44fec8149fbc17c86249725951e9",
            false),
        simple_seed(
            "n6.redgold.io",
            "020d55dcaed1ab7c1256e39d0acba37ba4f64c6e383a44d5a720a80b999077cb1b",
            "031b3b7c2c3ccf56994f684743128b444f302e450f9dd2a384424234f7fb0e71a2",
            false),
        simple_seed(
            "n7.redgold.io",
            "02f9a82dcf906be252dcf65569860a1f3f573928166917d621c38466c4ebcb685b",
            "02f76a329c8c0b80ba973b28c048dcfdfc58b8f992ac13d85c31463d64dfc1482d",
            false),
        simple_seed(
            "n8.redgold.io",
            "035c74e389a5677085ecf56e6678e4d704be6030cf74e02f01d59f58ede1fdfbec",
            "0364612f26001ff2c17fae5d889af58f5263b5a823fcd261dc525c0279710eb300",
            false),
        simple_seed(
            "n9.redgold.io",
            "0365c5b87014fed14b80a812e649a2be594d713fedb25e216f4426264b032b8ca1",
            "02be2bde0f08f8430fb593edbc7de749f3c56d16891089e563adddde26b3524aa6",
            false),

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