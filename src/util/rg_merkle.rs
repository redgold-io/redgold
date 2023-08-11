use itertools::Itertools;
use redgold_keys::util::{dhash, dhash_vec};

use redgold_schema::structs::HashType;
use redgold_keys::util::merge_hash;

use crate::schema::bytes_data;
use crate::schema::structs::{Hash, HashFormatType};
use crate::util;

// this is incredibly inefficient but is a placeholder for now.
#[derive(Debug)]
pub struct TempMerkleNode {
    parent: [u8; 32],
    left: [u8; 32],
    right: [u8; 32],
}

fn round(
    leafs: &Vec<[u8; 32]>,
    store_child_to_parent_map: &mut Option<&mut Vec<TempMerkleNode>>,
) -> (Vec<[u8; 32]>, Option<[u8; 32]>) {
    let mut remainder: Option<[u8; 32]> = None;
    let mut parent_hashes: Vec<[u8; 32]> = Vec::new();
    for chunk in leafs.chunks(2) {
        let left = chunk.get(0).unwrap();
        let right = chunk.get(1);
        match right {
            None => {
                remainder = Some(*left);
            }
            Some(r) => {
                let mut merged = left.to_vec();
                merged.extend(r.to_vec());
                let parent_hash = dhash_vec(&merged);
                println!(
                    "left {:?}, right {:?} merged {:?}",
                    hex::encode(left),
                    hex::encode(r),
                    hex::encode(parent_hash)
                );
                parent_hashes.push(parent_hash);
                match store_child_to_parent_map {
                    None => {}
                    Some(ref mut m) => m.push(TempMerkleNode {
                        parent: parent_hash,
                        left: *left,
                        right: *r,
                    }),
                }
            }
        }
    }
    return (parent_hashes, remainder);
}

// TODO: Update this function to use the new Hash struct
// i think this might not even be used, just delete after changing the reference.
pub fn build_root_simple(leafs: &Vec<Vec<u8>>) -> Hash {
    let res = leafs
        .iter()
        .map(|l| {
            let mut merged = [0u8; 32];
            merged[0..32].clone_from_slice(&*l);
            merged
        })
        .collect_vec();
    let res = build_root(&res, None, &mut None).to_vec();
    return Hash {
        bytes: bytes_data(res),
        // TODO: This is not correct but nothing is using it afaik?
        hash_format_type: HashFormatType::Sha3256 as i32,
        hash_type: HashType::ObservationMerkleRoot as i32,
    };
}

#[allow(unused_assignments)]
pub fn build_root(
    leafs: &Vec<[u8; 32]>,
    mut store_intermediate: Option<&mut Vec<[u8; 32]>>,
    mut store_child_to_parent_map: &mut Option<&mut Vec<TempMerkleNode>>,
) -> [u8; 32] {
    let mut active_round = leafs.clone();
    if active_round.len() == 1 {
        let input = active_round.get(0).unwrap();
        let root = dhash(input);
        match store_child_to_parent_map {
            None => {}
            Some(ref mut m) => m.push(TempMerkleNode {
                parent: root,
                left: input.clone(),
                right: input.clone(),
            }),
        }
        return root;
    }

    let mut remainder: Option<[u8; 32]> = None;
    let mut _round_idx = 0;
    while active_round.len() > 1 {
        let (this_round, this_remainder) = round(&active_round, &mut store_child_to_parent_map);
        active_round = this_round.clone();
        remainder = this_remainder;
        // if odd number of nodes, tack remainder onto end.
        if active_round.len() % 2 != 0 && remainder.is_some() {
            active_round.push(remainder.unwrap());
        }
        if active_round.len() > 1 {
            match store_intermediate {
                None => {}
                Some(ref mut inter) => {
                    inter.extend(this_round);
                }
            }
        }

        _round_idx += 1
    }

    let root = active_round.get(0).unwrap();
    return *root;
}

fn create_proof(nodes: Vec<TempMerkleNode>, hash: [u8; 32]) -> Vec<[u8; 32]> {
    let mut find_hash = hash;
    let mut proof: Vec<[u8; 32]> = Vec::new();
    loop {
        let node = nodes
            .iter()
            .find(|n| n.left == find_hash || n.right == find_hash);
        match node {
            None => {
                break;
            }
            Some(n) => {
                proof.push(n.left);
                proof.push(n.right);
                find_hash = n.parent;
            }
        }
    }
    return proof;
}

fn verify_proof(proof: Vec<[u8; 32]>, hash: [u8; 32]) -> bool {
    let first = proof.get(0).unwrap();
    let second = proof.get(1).unwrap();
    if !(*first == hash || *second == hash) {
        return false;
    }

    if proof.len() == 2 {
        return dhash(first) == *second;
    }

    // let mut parent: Option<[u8; 32]> = None;

    for chunk in proof.chunks(2) {
        let left = chunk.get(0).unwrap();
        match chunk.get(1) {
            None => {}
            Some(right) => {
                let mut merged = left.to_vec();
                merged.extend(right.to_vec());
                // let this_parent_hash = dhash_vec(&merged);
            }
        }
    }
    return true;
}

fn build_merkle_proof(
    root: [u8; 32],
    child_map: &Vec<TempMerkleNode>,
    leaf: [u8; 32],
) -> Vec<[u8; 32]> {
    let mut active_node = leaf.clone();
    let mut proof: Vec<[u8; 32]> = vec![];
    // info!("{:?}", child_map);
    while active_node != root {
        let n = child_map
            .iter()
            .find(|n| n.left == active_node || n.right == active_node)
            .unwrap();
        active_node = n.parent;
        proof.push(n.left);
        proof.push(n.right);
    }
    return proof;
}

#[allow(unused_assignments)]
fn verify_merkle_proof(root: [u8; 32], leaf: [u8; 32], proof: Vec<[u8; 32]>) -> bool {
    if proof.len() % 2 != 0 {
        return false;
    }
    let mut prev: Option<[u8; 32]> = None;
    for chunk in proof.chunks_exact(2) {
        let left = chunk[0];
        let right = chunk[1];
        let next_parent = merge_hash(left, right);
        match prev {
            Some(p) => {
                if !(p == left || p == right) {
                    return false;
                }
            }
            None => {
                if !(leaf == left || leaf == right) {
                    return false;
                }
                prev = Some(next_parent);
            }
        }
        prev = Some(next_parent);
    }
    prev == Some(root)
}
//
// pub fn build_root_and_proofs(leafs: Vec<Vec<u8>>) -> Vec<MerkleProof> {
//     let mut child_map: Vec<TempMerkleNode> = Vec::new();
//     let leafs_arr = leafs
//         .iter()
//         .map(|l| vec_to_fixed(l))
//         .collect::<Vec<[u8; 32]>>();
//     let root = build_root(&leafs_arr, None, &mut Some(&mut child_map));
//     let mut proofs: Vec<MerkleProof> = vec![];
//     for leaf in leafs {
//         let proof = build_merkle_proof(root, &child_map, vec_to_fixed(&leaf));
//         let leaves = proof
//             .iter()
//             .map(|p| MerkleProofElement {
//                 leaf_hash: p.to_vec(),
//             })
//             .collect::<Vec<MerkleProofElement>>();
//         proofs.push(MerkleProof {
//             root: root.clone().to_vec(),
//             leaves,
//         })
//     }
//     return proofs;
// }

// #[test]
// fn single_hash_merkle() {
//     util::init_logger().ok();
//     let h1 = util::dhash_str("h1");
//     let leafs = vec![h1.to_vec()];
//     let _res = build_root_and_proofs(leafs);
// }

// https://crypto.stackexchange.com/questions/2106/what-is-the-purpose-of-using-different-hash-functions-for-the-leaves-and-interna
#[test]
fn merkle() {
    let h1 = util::dhash_str("h1");
    let h2 = util::dhash_str("h2");
    let h3 = util::dhash_str("h3");

    let leafs = vec![h1, h2, h3];

    println!(
        "Original leafs: {:?}",
        leafs
            .clone()
            .iter()
            .map(|l| hex::encode(l))
            .collect::<Vec<String>>()
    );

    let mut intermediates: Vec<[u8; 32]> = Vec::new();

    // let mut child_map: HashMap<[u8; 32], [u8; 32]> = HashMap::new();
    let mut child_map: Vec<TempMerkleNode> = Vec::new();

    let root = build_root(&leafs, Some(&mut intermediates), &mut Some(&mut child_map));

    println!("Root: {:?}", hex::encode(root));
    println!(
        "Intermediates: {:?}",
        intermediates
            .iter()
            .map(hex::encode)
            .collect::<Vec<String>>()
    );
    println!(
        "Child map {:?}",
        child_map
            .iter()
            .map(
                |TempMerkleNode {
                     parent,
                     left,
                     right,
                 }| hex::encode(parent)
                    + " : "
                    + &*hex::encode(left)
                    + " : "
                    + &*hex::encode(right)
            )
            .collect::<Vec<String>>()
    );

    let proof = build_merkle_proof(root, &child_map, h1);

    verify_merkle_proof(root, h1, proof);

    /*

        Original leafs: ["bb9190bfcd8a7e580c8891546ae0884cdc074f229d0be99faef861c656a39838", "b69c39a39d0687427ce62b2e46e7e3616a9b7c8d176795b84a01bc50b13db8d4", "f919e6a760b51301c8ba3a218dfd94bae77fc72ecfb0ab8c4906271fff1d0c54"]
    left "bb9190bfcd8a7e580c8891546ae0884cdc074f229d0be99faef861c656a39838", right "b69c39a39d0687427ce62b2e46e7e3616a9b7c8d176795b84a01bc50b13db8d4" merged "8517ce70be886110b9b835600a9a537fd962eb0bdca43026f355729662aa9015"
    left "8517ce70be886110b9b835600a9a537fd962eb0bdca43026f355729662aa9015", right "f919e6a760b51301c8ba3a218dfd94bae77fc72ecfb0ab8c4906271fff1d0c54" merged "0ba6d1d9c89a4c423902860dee1ec31477349e86055ad93d7ac4ed21d39a5378"
    Root: "0ba6d1d9c89a4c423902860dee1ec31477349e86055ad93d7ac4ed21d39a5378"
    Intermediates: ["8517ce70be886110b9b835600a9a537fd962eb0bdca43026f355729662aa9015"]
    Child map ["8517ce70be886110b9b835600a9a537fd962eb0bdca43026f355729662aa9015 : bb9190bfcd8a7e580c8891546ae0884cdc074f229d0be99faef861c656a39838 : b69c39a39d0687427ce62b2e46e7e3616a9b7c8d176795b84a01bc50b13db8d4", "0ba6d1d9c89a4c423902860dee1ec31477349e86055ad93d7ac4ed21d39a5378 : 8517ce70be886110b9b835600a9a537fd962eb0bdca43026f355729662aa9015 : f919e6a760b51301c8ba3a218dfd94bae77fc72ecfb0ab8c4906271fff1d0c54"]

         */

    // let mut parent_map: HashMap<[u8; 32], Vec<[u8;32]>> = HashMap::new();
    //
    // for (k, v) in child_map {
    //     if parent_map.contains_key(&*v) {
    //         parent_map.get(&*v).unwrap().push(k);
    //     } else {
    //         parent_map.insert(v, vec![k]);
    //     }
    // }
    //

    //
    // println!(
    //     "Parent map {:?}",
    //     child_map
    //         .iter()
    //         .map(|(k, v)| hex::encode(k) + " : " + &*hex::encode(v))
    //         .collect::<Vec<String>>()
    // );
    //
}

// fn create_proof(
//     leafs: Vec<[u8; 32]>, intermediate: Vec<[u8; 32]>, root: [u8; 32], leaf: [u8; 32]) {
//
//     let mut proof: Vec<[u8; 32]> = Vec::new();
//
//     // proof.push(leaf);
//     /*
//     should i do this purely on indexes? if i know the index then i know the intermediate location?
//      */
//
//     let mut remainder: Option<[u8; 32]> = None;
//     let mut remainder_proof: Option<[u8; 32]> = None;
//     let mut active_chunk_idx: usize = 0;
//
//     for (chunk_idx, chunk) in leafs.chunks(2).enumerate() {
//         let left = chunk.get(0).unwrap();
//         let right = chunk.get(1);
//         match right {
//             None => {
//                 if left == leaf {
//                     remainder_proof = Some(*left);
//                     active_chunk_idx = chunk_idx;
//                 }
//                 remainder = Some(*left);
//             }
//             Some(r) => {
//                 if left == leaf || right == leaf {
//                     proof.push(*left);
//                     proof.push(*r);
//                     active_chunk_idx = chunk_idx;
//                 }
//             }
//         }
//     }
// }
// fn create_proof2(
//     leafs: Vec<[u8; 32]>, intermediate: Vec<[u8; 32]>, root: [u8; 32], leaf_idx: usize, leaf: [u8; 32]) {
//     let mut proof: Vec<[u8; 32]> = Vec::new();
//
//     assert!(leaf_idx < leafs.len());
//
//     let mut current_idx = leaf_idx.clone();
//     let mut current_slice = leafs.clone();
//     let mut remainder: Option<[u8; 32]> = None;
//     let mut current_offset = 0;
//
//     // loop below after initialization done.
//
//     while current_max > 0 {
//         let mut current_max = current_slice.len() - 1;
//         if current_idx % 2 == 0 {
//             // we are the left node, there may be a right or not
//             let current_node = current_slice.get(current_idx).unwrap();
//             if current_idx + 1 > current_max {
//                 // there is no right node, carry remainder
//                 remainder = Some(*current_node);
//             } else {
//                 let left = current_node;
//                 let right = current_slice.get(current_idx + 1).unwrap();
//                 proof.push(*left);
//                 proof.push(*right);
//             }
//         } else {
//             // we are the right node, and there must be a left;
//             let left = current_slice.get(current_idx - 1).unwrap();
//             proof.push(*left);
//             let right = current_slice.get(current_idx).unwrap();
//             proof.push(*right);
//         }
//         let mut next_max = 0;
//         if current_max % 2 == 0 {
//             next_max = (current_max - 1) / 2;
//         } else {
//             next_max = current_max / 2;
//         }
//         current_slice = intermediate[current_offset..next_max].to_owned().clone();
//         current_offset += current_max;
//
//
//
//     }
// }
