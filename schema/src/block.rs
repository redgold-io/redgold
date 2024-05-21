use crate::structs::{Block, ErrorInfo, StructMetadata};
use crate::SafeOption;
use crate::HashClear;
use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::helpers::with_metadata_hashable::WithMetadataHashableFields;

impl HashClear for Block {
    fn hash_clear(&mut self) {
        self.transactions.clear();
        if let Some(sm) = self.struct_metadata.as_mut() {
            sm.hash_clear()
        }
    }
}

impl WithMetadataHashableFields for Block {
    fn struct_metadata_opt(&mut self) -> Option<&mut StructMetadata> {
        self.struct_metadata.as_mut()
    }

    fn struct_metadata_opt_ref(&self) -> Option<&StructMetadata> {
        self.struct_metadata.as_ref()
    }
}

impl Block {

    pub fn time(&self) -> Result<i64, ErrorInfo> {
        Ok(self
            .struct_metadata
            .safe_get()?
            .time
            .safe_get()?
            .clone()
        )
    }

    // pub fn from(transactions: Vec<Transaction>, last_time: i64, ) {
    //     if let Some(txa) = txs {
    //         // TODO: re-query observation edge here.
    //         let leafs = transactions
    //             .clone()
    //             .iter()
    //             .map(|e| e.hash_vec().clone())
    //             .collect_vec();
    //         let mut block = Block {
    //             // TODO: use a real merkle root here
    //             merkle_root: Some(rg_merkle::build_root_simple(&leafs)),
    //             transactions: vec, // TODO: can leave this blank and enforce it properly
    //             // to remove the clone on hash calculation? That's clever do it as part
    //             // of a constructor function.
    //             struct_metadata: struct_metadata(last_time as i64),
    //             previous_block_hash: self.last_block.hash.clone(),
    //             metadata: None,
    //             hash: None,
    //             height: self.last_block.height + 1,
    //         }
    //             .with_hash();
    // }
}

#[test]
fn verify_hash_equivalent_schema_change() {
    let block = Block {
        merkle_root: None,
        transactions: vec![],
        struct_metadata: None,
        previous_block_hash: None,
        metadata: None,
        height: 0,
    };
    // before adding 'test_field: Option<String>'
    // 1440a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26
    println!("Hash hex empty block: {}", block.hash_hex().expect(""));
    // same hash after adding test_field: None to schema, verifying we can evolve schema while
    // keeping hashes consistent so long as all fields are None at first.
}
