use itertools::Itertools;
use crate::structs::{NodeMetadata, PartitionInfo, PublicKey};

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
pub fn xor_conv_distance(query_hash: &Vec<u8>, peer_id: &Vec<u8>) -> i64 {

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

pub trait XorConvDistanceSubset<T> {
    fn xor_conv_distance_subset<F>(
        &self, other: &Vec<u8>, partition_func: F
    ) -> Vec<&T>
        where F: FnOnce(&PartitionInfo) -> Option<i64> + Copy;
}

impl XorConvDistanceSubset<NodeMetadata> for Vec<NodeMetadata> {
    fn xor_conv_distance_subset<F>(
        &self, other: &Vec<u8>, partition_func: F
    ) -> Vec<&NodeMetadata>
    where F: FnOnce(&PartitionInfo) -> Option<i64> + Copy {
        let mut res = self.iter()
            .filter_map(|n| n.public_key.as_ref().map(|p| (p, n)))
            .filter_map(|(p, n)| p.bytes.as_ref().map(|b| (b.value.clone(), n)))
            .map(|(p, n)| (xor_conv_distance(other, &p), n))
            .filter(|(p, n)|
                n.partition_info.as_ref().and_then(|i| partition_func(i)).map(|d| d < *p).unwrap_or(true))
            .sorted_by(|(p1, _), (p2, _)| p1.cmp(p2))
            .map(|(p, n)| n)
            .collect_vec();
        res
    }
}

#[test]
pub fn xor_test() {

}
