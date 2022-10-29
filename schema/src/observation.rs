use itertools::Itertools;
use crate::structs::{Observation, ObservationMetadata};
use crate::{struct_metadata_new, Hash, HashClear, StructMetadata, TestConstants,
    WithMetadataHashable, WithMetadataHashableFields,
};

impl HashClear for Observation {
    fn hash_clear(&mut self) {
        self.hash = None;
    }
}

impl WithMetadataHashableFields for Observation {
    fn set_hash(&mut self, hash: Hash) {
        self.hash = Some(hash);
    }

    fn stored_hash_opt(&self) -> Option<Hash> {
        self.hash.clone()
    }

    fn struct_metadata_opt(&self) -> Option<StructMetadata> {
        self.struct_metadata.clone()
    }
}

impl HashClear for ObservationMetadata {
    fn hash_clear(&mut self) {
        self.hash = None;
    }
}

impl WithMetadataHashableFields for ObservationMetadata {
    fn set_hash(&mut self, hash: Hash) {
        self.hash = Some(hash);
    }

    fn stored_hash_opt(&self) -> Option<Hash> {
        self.hash.clone()
    }

    fn struct_metadata_opt(&self) -> Option<StructMetadata> {
        self.struct_metadata.clone()
    }
}

impl Observation {
    pub fn leafs(&self) -> Vec<Vec<u8>> {
        self.observations
            .iter()
            .map(|r| r.hash_vec())
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
            .map(|r| r.hash())
            .collect_vec()
    }

    pub fn leafs_vec(&self) -> Vec<Vec<u8>> {
        self.leafs()
            .iter()
            .map(|r| r.to_vec())
            .collect::<Vec<Vec<u8>>>()
    }

}

impl ObservationMetadata {
    pub fn default() -> Self {
        let tc = TestConstants::new();
        Self {
            observed_hash: Some(tc.hash_vec.into()),
            hash: None,
            hash_type: 0,
            state: None,
            struct_metadata: struct_metadata_new(),
        }
    }
}
