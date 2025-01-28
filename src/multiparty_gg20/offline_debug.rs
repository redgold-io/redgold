use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs;
use redgold_schema::structs::InitiateMultipartyKeygenRequest;
use std::path::PathBuf;

#[ignore]
#[test]
fn debug_load_offline_shares() {
    for line in std::fs::read_to_string(PathBuf::from("exportedData.csv")).unwrap().split("\n") {
        /*
        sqlite3 /root/.rg/dev/data_store.sqlite "SELECT
        room_id, keygen_time, hex(keygen_public_key), hex(host_public_key),
        self_initiated, hex(local_share), hex(initiate_keygen) FROM multiparty;" > /root/.rg/dev/exportedData.csv
         */
        if line.len() == 0 {
            continue;
        }
        let mut parts = line.split("|");
        let room_id = parts.next().unwrap();
        let keygen_time = parts.next().unwrap();
        let keygen_public_key = structs::PublicKey::from_hex(parts.next().unwrap()).unwrap();
        let host_public_key = structs::PublicKey::from_hex(parts.next().unwrap()).unwrap();
        let self_initiated = parts.next().unwrap().parse::<u8>().unwrap();
        let local_share = String::from_utf8(hex::decode(parts.next().unwrap()).unwrap()).unwrap();
        let initiate_keygen = InitiateMultipartyKeygenRequest::proto_deserialize(hex::decode(parts.next().unwrap()).unwrap()).unwrap();
        println!("{} {} {} {} {} {} {:?}", room_id, keygen_time, keygen_public_key, host_public_key, self_initiated, local_share, initiate_keygen);
    }
}