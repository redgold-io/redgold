
pub fn xor_distance(v1: &Vec<u8>, v2: &Vec<u8>) -> i64 {
    let xor_value: Vec<u8> = v1
        .iter()
        .zip(v2.iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    let distance: i64 = xor_value.iter().map(|&byte| i64::from(byte)).sum();
    distance
}

// Really need to do this as AsRef<u8;32>
pub fn xorf_distance(query_hash: &Vec<u8>, peer_id: &Vec<u8>) -> i64 {

    let mut concat = query_hash.clone();
    concat.extend(peer_id.clone());
    // TODO: hash of concat

    let xor_value: Vec<u8> = query_hash
        .iter()
        .zip(concat.iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    let distance: i64 = xor_value.iter().map(|&byte| i64::from(byte)).sum();
    distance
}

#[test]
pub fn xor_test() {

}
