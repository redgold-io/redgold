use crate::proto_serde::ProtoSerde;
use crate::structs::{Address, NetworkEnvironment, PeerId, PublicKey, Seed, TrustData};
use crate::util::times::current_time_millis;
use itertools::Itertools;


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

pub fn get_all_hardcoded_seeds() -> Vec<Seed> {
    vec![
        simple_seed(
            "n0.redgold.io",
            "0a250a230a21028f78d7ac63a35087092f05981a115bf42b6afc31fd97eb4107b1e2b73d27f98a",
            "0a230a2103a59d904435a72b9d97bddece79692bef51a0fb030b277deb9dc69b75ebc38c6f",
            false),
        simple_seed(
            "n1.redgold.io",
            "0a250a230a2102e81515dfc521dbfa5ed4ea952f8d080612f1782063f97fe5bbe858c470710027",
            "0a230a2103219cb61b1463c1c507501f6621d18092cf44ee9c49c8e998a56c638da5b03b07",
            false),
        simple_seed(
            "n2.redgold.io",
            "0a250a230a2103dc0949cb7027639a8500b43986e34fd5eff774d968cb84133160b3b330c259aa",
            "0a230a2103413bfd6768962e291e57047459baff1be8869e185b75c06c99fde66899aeb787",
            false),
        simple_seed(
            "n3.redgold.io",
            "0a250a230a21022ac18c9106276a154abe929b1bb0f3b7004db924d62a2d37fc9c2a3f4f996c28",
            "0a230a210274a5a0382173ab16706782caaa9c9b4c2bca8dcd5d6321460c8e5619be830913",
            false),
        simple_seed(
            "n4.redgold.io",
            "0a250a230a210322841b5833ddb734ecec64ffd03250141a22d34978626d8db670f5eb21fef14f",
            "0a230a2103bac75d3501cfe5893324cc79bbfb91f471712547dc30b26b34f7fedcb05ce1fe",
            false),
        simple_seed(
            "n5.redgold.io",
            "0a250a230a21023e4471aa6ad46d04e6e92642b5d6b52ef968022568ee39241cdb6daea0c888a0",
            "0a230a21022a557a6f3b27a5b734665231d9eca9b38cdb44fec8149fbc17c86249725951e9",
            false),
        simple_seed(
            "n6.redgold.io",
            "0a250a230a21020d55dcaed1ab7c1256e39d0acba37ba4f64c6e383a44d5a720a80b999077cb1b",
            "0a230a21031b3b7c2c3ccf56994f684743128b444f302e450f9dd2a384424234f7fb0e71a2",
            false),
        simple_seed(
            "n7.redgold.io",
            "0a250a230a2102f9a82dcf906be252dcf65569860a1f3f573928166917d621c38466c4ebcb685b",
            "0a230a2102f76a329c8c0b80ba973b28c048dcfdfc58b8f992ac13d85c31463d64dfc1482d",
            false),
        // simple_seed(
        //     "n8.redgold.io",
        //     "0a250a230a21035c74e389a5677085ecf56e6678e4d704be6030cf74e02f01d59f58ede1fdfbec",
        //     "0a230a210364612f26001ff2c17fae5d889af58f5263b5a823fcd261dc525c0279710eb300",
        //     false),
        // simple_seed(
        //     "n9.redgold.io",
        //     "0a250a230a210365c5b87014fed14b80a812e649a2be594d713fedb25e216f4426264b032b8ca1",
        //     "0a230a2102be2bde0f08f8430fb593edbc7de749f3c56d16891089e563adddde26b3524aa6",
        //     false),
        simple_seed(
            "n0.redgold.io",
            "0a250a230a2102ef6e72f7160edaf940d04790cf4e6ba9e6380672916cdb2ed01e25af8be849e7",
            "0a230a21032b1b73342cc848b41d9b5a53ed67730be78222a6abf22650a22a4c079743d3f8",
            true),
        simple_seed(
            "n1.redgold.io",
            "0a250a230a210241f3e4dacd61bacc6bc4ea2731ac2d66431e676cc96b9dc238c362eb785b08ed",
            "0a230a21024e866d93f75c46a9e3070d8d35af36946597ffaeced201b0e7158ac85008699d",
            true),
        simple_seed(
            "n2.redgold.io",
            "0a250a230a2103bf103bb4fab013c55bfeaf386466a82d82fe61e5a453c3a9d6aca3e4dfd246b2",
            "0a230a2103316ce5f955360c9c4e1597bc041d0f14fa3d4b9691209d3beb64eb246ac44102",
            true),
        simple_seed(
            "n3.redgold.io",
            "0a250a230a21021158dc186923ab8a1dba1b19e8cd5017e0d104ac7ed203723464218b8c6663d2",
            "0a230a210217b3a710fbcc8ac68ad7b5a7e5910b189e071348cf560c3db853bef4afcdafc6",
            true),
        simple_seed(
            "n4.redgold.io",
            "0a250a230a2103a744d97d4b509edc6020985720479f5804d630d6a9d707bf3fffd6790a5aa4ba",
            "0a230a2103039218939094eeb8acafecb0a6387d1024b411d3b6341a3b1e73001b6d2e11f3",
            true),
        simple_seed(
            "n5.redgold.io",
            "0a250a230a2102661fd340f7ab50e4d33f23fedd040cafd70ade7cd0dec2eecbcdb6d6533597fd",
            "0a230a2103787246687366699530545a73bfd5d66e6b5186924f363fe4bce27161e92de56f",
            true),
        simple_seed(
            "n6.redgold.io",
            "0a250a230a2103b0b884f2f4d4df42b85c3c43acb8a122ae051ca0ceb735b791198bf9a6629a28",
            "0a230a210207df4c1553162abcb9d018528d89619b77241676d9c352105fd92fd20cfa19d6",
            true),
        simple_seed(
            "n7.redgold.io",
            "0a250a230a2103ac58ceacf822ef6208aeef0c439b66311cf7fc1108412a6d3378049977d939ad",
            "0a230a2103ebcb86c597c5ab059077fe4bdfe7fe44dd9d1b6b63ffc2fb2af7e5448317cccb",
            true),
        // simple_seed(
        //     "n0.redgold.io",
        //     "0a250a230a210380584c41f3185a656d7af96d5f0c2d4dc423dd67a0eae96c3be45d94d2afdf8b",
        //     "0a230a21029ddcbe05dd9430579bd280b190251ac50e840d5574c90454458fed53dd0b3118",
        //     true),
        // simple_seed(
        //     "n1.redgold.io",
        //     "0a250a230a2102c38d01c59be615b53e80e1f31ed31a368be0ba45ed8360b94e52f293c561d22e",
        //     "0a230a21024a345465ee1bcb042ce0fdf7f5686caae897681db8ca31a49d2bfba761ec49b5",
        //     true),
        // simple_seed(
        //     "n2.redgold.io",
        //     "0a250a230a21032b9aa677503a43565086d16a873b2489c40f35d0aa1f27d5db798ad1b256880e",
        //     "0a230a2102c20b0b607c4864ac19681a4ec05b61185e674275bb8be84c2e49a4dfb7763839",
        //     true),
        // simple_seed(
        //     "n3.redgold.io",
        //     "0a250a230a2103cfa336db912bc5aec088e92f7b45e9de5f6b9f10fa801f889c2c6a05718ad442",
        //     "0a230a210272af680bdab9dce33652ab488e66a5b8e6807a5ad1306ba34afa878229fafbd3",
        //     true),
        // simple_seed(
        //     "n4.redgold.io",
        //     "0a250a230a2103efaf6dbf77f2cdc158adb49b391bed72099cb4cb462cb3588218b4fe317d383f",
        //     "0a230a210335a2e07f3fd60b6bcee6ae2e2699c5852ae59af456ca229509f2a60ab3ba1048",
        //     true),
        // simple_seed(
        //     "n5.redgold.io",
        //     "0a250a230a21034c787a31cb693c156a0377df2b37139de03ac278fa50e35f28d4d530d0a7b2f7",
        //     "0a230a2103c52d3f451c8f8a255f4600686ea7203524519d30ef284676ef3c9bc76bb44ff2",
        //     true),
        // simple_seed(
        //     "n6.redgold.io",
        //     "0a250a230a2102537531c05b49853f65f7818c2fed4b503e1851faae93ff4dedae64fd746d91b4",
        //     "0a230a21029d4e040957387d07be9ac566e19b8de1dc8c8e03b5d991a4aa2c85b66b88847e",
        //     true),
        // simple_seed(
        //     "n7.redgold.io",
        //     "0a250a230a2103e193a617f42b30c349f5d27d176c8157aee480472f3217604e3c47d2e99b6ef8",
        //     "0a230a21030d2402a8e3f5145d0e934992efb31c49664d10d0d0831096d02849436c382cb2",
        //     true),
    ]
}

pub fn get_seeds_by_env(env: &NetworkEnvironment) -> Vec<Seed> {
    get_all_hardcoded_seeds().into_iter()
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