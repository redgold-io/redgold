use itertools::Itertools;
use crate::structs::{Hash, NodeMetadata, PartitionInfo, PublicKey};

pub fn xor_distance(v1: &Vec<u8>, v2: &Vec<u8>) -> i64 {
    let xor_value: Vec<u8> = v1
        .iter()
        .zip(v2.iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    let distance: i64 = xor_value.iter().map(|&byte| i64::from(byte)).sum();
    distance
}

pub fn xorf_conv_distance(query_hash: &Vec<u8>, peer_marker: &Vec<u8>) -> i64 {

    // Helps prevent pre-generating TX hash to be near specific peer markers
    let mut concat = query_hash.clone();
    concat.extend(peer_marker.clone());
    // 32 bytes
    let merged = Hash::digest(concat).vec();

    // Helps prevent pre-generating peer markers to be near one another.
    let mut concat2 = peer_marker.clone();
    concat2.extend(query_hash.clone());

    let merged2 = Hash::digest(concat2).vec();

    let xor_value: Vec<u8> = merged
        .iter()
        .zip(merged2.iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    let distance: i64 = xor_value.iter().map(|&byte| i64::from(byte)).sum();
    distance
}

pub fn xorfc_hash(query_hash: &Hash, pk: &PublicKey) -> i64 {
    let query_hash = query_hash.vec();
    let pk_bytes = pk.bytes().expect("bytes");
    xorf_conv_distance(&query_hash, &pk_bytes)
}

pub trait XorfConvDistanceSubset<T> {
    fn xorf_conv_distance_subset<F>(
        &self, other: &Vec<u8>, partition_func: F
    ) -> Vec<&T>
        where F: FnOnce(&PartitionInfo) -> Option<i64> + Copy;
}

impl XorfConvDistanceSubset<NodeMetadata> for Vec<NodeMetadata> {
    fn xorf_conv_distance_subset<F>(
        &self, other: &Vec<u8>, partition_func: F
    ) -> Vec<&NodeMetadata>
    where F: FnOnce(&PartitionInfo) -> Option<i64> + Copy {
        let res = self.iter()
            .filter_map(|n| n.public_key.as_ref().map(|p| (p, n)))
            .filter_map(|(p, n)| p.bytes.as_ref().map(|b| (b.value.clone(), n)))
            .map(|(p, n)| (xorf_conv_distance(other, &p), n))
            .filter(|(p, n)|
                n.partition_info.as_ref().and_then(|i| partition_func(i)).map(|d| d < *p).unwrap_or(true))
            .sorted_by(|(p1, _), (p2, _)| p1.cmp(p2))
            .map(|(_p, n)| n)
            .collect_vec();
        res
    }
}

#[test]
pub fn xor_test() {
}
