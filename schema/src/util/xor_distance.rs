
pub fn xor_distance(v1: &Vec<u8>, v2: &Vec<u8>) -> i64 {
    let xor_value: Vec<u8> = v1
        .iter()
        .zip(v2.iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    let distance: i64 = xor_value.iter().map(|&byte| i64::from(byte)).sum();
    distance
}