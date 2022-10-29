use prost::DecodeError;
use crate::structs::MerkleProof;

use prost::Message;
use crate::{ErrorInfo, SafeOption};

impl MerkleProof {
    pub fn proto_serialize(&self) -> Vec<u8> {
        return self.encode_to_vec();
    }

    pub fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        return MerkleProof::decode(&*bytes);
    }
}


impl MerkleProof {
    pub fn verify(&self) -> Result<(), ErrorInfo> {
        let len = self.nodes.len();
        for i in (0..len).step_by(2) {
            let left = self.nodes[i].clone();
            let right = self.nodes[i + 1].clone();
            let parent = left.merkle_combine(right.clone());
            let parent_vec = parent.vec();

            if i == 0 {
                let leaf = self.leaf.safe_get_msg("leaf missing")?.vec();
                if !(leaf == left.vec() || leaf == right.vec()) {
                    return Err(ErrorInfo::error_info("Leaf not found at proof start"));
                }
            }

            if i+1 == (len-1) {
                if parent_vec != self.root.safe_get_msg("Root")?.vec() {
                    return Err(ErrorInfo::error_info("Last intermediate hash merge does not match root"));
                }
            } else {
                let next_left = self.nodes[i + 2].clone();
                let next_right = self.nodes[i + 3].clone();
                let next = vec![next_left.vec(), next_right.vec()];
                if !next.contains(&parent_vec) {
                    return Err(ErrorInfo::error_info(
                        format!("Intermediate hash mismatch on parent: {} with next_left: {} next_right: {}",
                                parent.hex(), next_left.hex(), next_right.hex())));

                }
            }
        }
        Ok(())
    }
}

