// Internal structure. UtxoEntry is not consistent because it maps exactly to database schema, while
// this is only used for in-memory ID entries
pub struct OldUtxoId {
    pub transaction_hash: Vec<u8>,
    pub output_index: i64,
}

impl OldUtxoId {
    pub fn from(id: Vec<u8>) -> OldUtxoId {
        let len = id.len();
        let split = len - 8;
        let mut output_index_array = [0u8; 8];
        let output_id_vec = &id[split..len];
        output_index_array.clone_from_slice(&output_id_vec);
        let output_index = i64::from_le_bytes(output_index_array);
        let transaction_hash = id[0..split].to_vec();
        OldUtxoId {
            transaction_hash,
            output_index,
        }
    }
    pub fn coin_id(&self) -> Vec<u8> {
        let mut merged: Vec<u8> = vec![];
        merged.extend(self.transaction_hash.clone());
        merged.extend(self.output_index.to_le_bytes().to_vec());
        merged
        // pub fn id_to_values(id: &Vec<u8>) -> (Vec<u8>, u32) {
        //     let vec = id[32..36].to_owned();
        //     let mut output_index = [0u8; 4];
        //     output_index.clone_from_slice(&vec);
        //     let value = u32::from_le_bytes(output_index);
        //     return (id[0..32].to_owned(), value);
        // }
    }
}
