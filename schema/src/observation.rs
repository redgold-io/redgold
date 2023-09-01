use itertools::Itertools;
use crate::structs::{Observation, ObservationMetadata, ObservationProof, Proof};
use crate::{Hash, HashClear, ProtoHashable, struct_metadata_new, StructMetadata, util, WithMetadataHashable, WithMetadataHashableFields};

//
// impl HashClear for Observation {
//     fn hash_clear(&mut self) {
//         for x in self.struct_metadata_opt() {
//             x.hash_clear();
//         }
//     }
// }
//
// impl WithMetadataHashableFields for Observation {
//     fn struct_metadata_opt(&mut self) -> Option<&mut StructMetadata> {
//         self.struct_metadata.as_mut()
//     }
//
//     fn struct_metadata_opt_ref(&self) -> Option<&StructMetadata> {
//         self.struct_metadata.as_ref()
//     }
// }

impl HashClear for ObservationMetadata {
    fn hash_clear(&mut self) {
        if let Some(s) = self.struct_metadata.as_mut() {
            s.hash_clear();
        }
    }
}

impl WithMetadataHashableFields for ObservationMetadata {
    fn struct_metadata_opt(&mut self) -> Option<&mut StructMetadata> {
        self.struct_metadata.as_mut()
    }

    fn struct_metadata_opt_ref(&self) -> Option<&StructMetadata> {
        self.struct_metadata.as_ref()
    }
}

impl Observation {
    pub fn leafs(&self) -> Vec<Vec<u8>> {
        self.observations
            .iter()
            .map(| r| r.clone().hash_vec())
            /*
                   old code
            pub fn metadata_hash(&self) -> [u8; 32] {
                return crate::util::dhash_vec(&self.proto_serialize());
            }
                     */
            .collect::<Vec<Vec<u8>>>()
    }

    pub fn leafs_hash(&self) -> Vec<Hash> {
        self.observations
            .iter()
            .map(|r| r.hash_or())
            .collect_vec()
    }

    pub fn leafs_vec(&self) -> Vec<Vec<u8>> {
        self.leafs()
            .iter()
            .map(|r| r.to_vec())
            .collect::<Vec<Vec<u8>>>()
    }

    pub fn build_observation_proofs(&self, obs_tx_hash: &Hash, proof: &Proof) -> Vec<ObservationProof> {
        let mut res = vec![];
        let leafs = self.leafs_hash();
        let merkle_tree = util::merkle::build_root(leafs.clone()).expect("merkle failure");
        // info!("Store observation leafs len={:?}", leafs.len());
        for observation_metadata in &self.observations {
            let hash = observation_metadata.hash_or();
            let merkle_proof = merkle_tree.proof(hash.clone());
            let mut op = ObservationProof::default();
            op.metadata = Some(observation_metadata.clone());
            op.merkle_proof = Some(merkle_proof);
            op.proof = Some(proof.clone());
            op.observation_hash = Some(obs_tx_hash.clone());
            res.push(op);
        };
        res
    }
    //
    // pub fn signable_hash(&self) -> Hash {
    //     let mut s = self.clone();
    //     s.proof = None;
    //     s.calculate_hash()
    // }

}
