use itertools::Itertools;
use crate::structs::{NetworkEnvironment, Seed, TrustData, TrustLabel};


#[derive(Debug, serde::Deserialize)]
struct SeedCsvRecord {
    external_address: String,
    network_environment: String,
    score: f64,
}


fn parse_seeds_csv_resource() -> Vec<SeedCsvRecord> {
    let str = include_str!("resources/seeds/seeds.csv");
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

fn get_seeds() -> Vec<Seed> {
    let csv_records = parse_seeds_csv_resource();
    // TODO: Embed any additional data in code here afterwards.
    csv_records.iter().map(|r| {
        let mut s = Seed::default();
        s.external_address = r.external_address.clone();
        s.environments = vec![NetworkEnvironment::parse(r.network_environment.clone()) as i32];
        let mut t = TrustData::default();
        t.label = r.score;
        s.trust = vec![t];
        s
    }).collect_vec()
}

#[test]
fn debug() {
    parse_seeds_csv_resource();
    assert!(get_seeds().len() >= 3);
}