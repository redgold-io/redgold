use redgold_keys::address_external::ToEthereumAddress;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::seeds::get_seeds_by_env;
use redgold_schema::structs::NetworkEnvironment;

#[tokio::test]
pub async fn debug() {
    let seeds = get_seeds_by_env(&NetworkEnvironment::Dev);
    for s in seeds.get(0) {
        println!("{}", s.json_or());
        println!("{}", s.clone().public_key.unwrap().to_ethereum_address().unwrap());
    }
}