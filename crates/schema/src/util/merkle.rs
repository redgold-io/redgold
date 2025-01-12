use std::collections::HashMap;
use crate::structs::{ErrorInfo, Hash, MerkleProof};

#[derive(Debug, Clone)]
pub struct MerkleEntry {
    parent: Hash,
    left: Hash,
    right: Hash,
}

#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub root: Hash,
    pub entries: Vec<MerkleEntry>,
    pub parent_lookup: HashMap<Vec<u8>, MerkleEntry>,
    pub child_to_parent: HashMap<Vec<u8>, MerkleEntry>,
}

impl MerkleTree {

    pub fn new(entries: Vec<MerkleEntry>) -> Self {

        let mut parent_lookup: HashMap<Vec<u8>, MerkleEntry> = HashMap::new();
        let mut child_to_parent: HashMap<Vec<u8>, MerkleEntry> = HashMap::new();
        for entry in &entries {
            parent_lookup.insert(entry.parent.vec(), entry.clone());
            child_to_parent.insert(entry.left.vec(), entry.clone());
            child_to_parent.insert(entry.right.vec(), entry.clone());
        }

        let x = entries.last().expect("Missing root").clone();
        Self {
            root: x.parent,
            entries,
            parent_lookup,
            child_to_parent
        }
    }

    pub fn proof(&self, leaf: Hash) -> MerkleProof {
        let mut merkle_nodes: Vec<Hash> = vec![];
        let mut active = leaf.clone();
        loop {
            let k = active.vec();
            let entry = self.child_to_parent.get(&k);
            if let Some(e) = entry {
                merkle_nodes.push(e.left.clone());
                merkle_nodes.push(e.right.clone());
                active = e.parent.clone();
            } else {
                break;
            }
        }
        MerkleProof{
            root: Some(self.root.clone()),
            leaf: Some(leaf.clone()),
            nodes: merkle_nodes
        }
    }
}

pub fn pad(hashes: &mut Vec<Hash>) {
    if hashes.len() % 2 == 1 {
        hashes.push(hashes.last().unwrap().clone());
    }
}

pub fn build_root(leafs_original: Vec<Hash>) -> Result<MerkleTree, ErrorInfo> {

    let mut leafs = leafs_original.clone();

    if leafs.is_empty() {
        return Err(ErrorInfo::error_info("Merkle tree build must have at least one leaf"));
    }

    let mut entries: Vec<MerkleEntry> = vec![];
    let mut done = false;

    while !done {
        pad(&mut leafs);
        let mut new_leafs: Vec<Hash> = vec![];
        for i in (0..leafs.len()).step_by(2) {
            let left = leafs[i].clone();
            let right = leafs[i + 1].clone();
            let parent = left.merkle_combine(right.clone());
            new_leafs.push(parent.clone());
            entries.push(MerkleEntry {
                parent,
                left,
                right,
            });
        }
        if new_leafs.len() == 1 {
            done = true
        }
        leafs = new_leafs;
    }
    
    Ok(MerkleTree::new(entries))
}