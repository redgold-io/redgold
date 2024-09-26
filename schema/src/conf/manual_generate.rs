use std::fs;
use crate::config_data::ConfigData;
use crate::helpers::easy_json::EasyJsonDeser;
use crate::local_stored_state::LocalStoredState;

#[test]
fn load_schema_dump() {
    let res = fs::read_to_string("../local_stored_state.json").unwrap();
    let dec = res.json_from::<LocalStoredState>().unwrap();
    let mut conf = ConfigData::default();
    conf.local_stored_state = dec;
    let toml = toml::to_string_pretty(&conf).unwrap();
    println!("{}", toml);

}