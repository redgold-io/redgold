//
// pub struct FixedIdConvert {
//     id: [u8; 36],
// }
//
// fn u32_to_vec(value: u32) -> Vec<u8> {
//     return value.to_le_bytes().to_vec();
// }
//
// impl FixedIdConvert {
//     pub(crate) fn from_values(hash: &[u8; 32], value: u32) -> FixedIdConvert {
//         let bytes = value.to_le_bytes();
//         let mut merged = [0u8; 36];
//         merged[0..32].clone_from_slice(&*hash);
//         merged[32..36].clone_from_slice(&bytes);
//         return FixedIdConvert { id: merged };
//     }
//
//     fn to_values(&self) -> ([u8; 32], u32) {
//         let mut hash = [0u8; 32];
//         let mut output_index = [0u8; 4];
//         hash.clone_from_slice(&self.id[0..32]);
//         output_index.clone_from_slice(&self.id[32..36]);
//         return (hash, u32::from_le_bytes(output_index));
//     }
// }

use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::structs::{Address, CurrencyAmount, Hash, Input, Output, UtxoEntry, UtxoId};
use crate::{RgResult, SafeOption, Transaction};

impl UtxoEntry {

    pub fn address(&self) -> RgResult<&Address> {
        let o = self.output.safe_get_msg("Missing address from utxo entry")?;
        o.address.safe_get_msg("Missing address from utxo entry")
    }
    pub fn utxo_id(&self) -> RgResult<&UtxoId> {
        self.utxo_id.safe_get_msg("Missing id from utxo entry")
    }

    pub fn amount(&self) -> u64 {
        return self.output.as_ref().unwrap().amount();
    }

    pub fn opt_amount(&self) -> Option<CurrencyAmount> {
        self.output.as_ref().and_then(|o| o.opt_amount_typed())
    }

    pub fn height(&self) -> RgResult<i64> {
        let h = self.output.as_ref()
            .and_then(|o| o.data.as_ref())
            .and_then(|o| o.height);
        h.safe_get_msg("Missing height on utxo output").cloned()
    }

    // pub fn id_from_values(hash: &Vec<u8>, value: &Vec<u8>) -> Vec<u8> {
    //     let mut merged: Vec<u8> = Vec::new();
    //     merged.extend(hash);
    //     merged.extend(value);
    //     return merged;
    // }

    // pub fn id_from_fixed_values(hash: &[u8; 32], value: u32) -> Vec<u8> {
    //     return UtxoEntry::id_from_fixed(&FixedIdConvert::from_values(hash, value));
    // }
    //
    // pub(crate) fn id_from_fixed(fixed: &FixedIdConvert) -> Vec<u8> {
    //     let (hash, index) = fixed.to_values();
    //     return UtxoEntry::id_from_values(&hash.to_vec(), &index.to_le_bytes().to_vec());
    // }
    //
    // pub fn id_to_values(id: &Vec<u8>) -> (Vec<u8>, u32) {
    //     let vec = id[32..36].to_owned();
    //     let mut output_index = [0u8; 4];
    //     output_index.clone_from_slice(&vec);
    //     let value = u32::from_le_bytes(output_index);
    //     return (id[0..32].to_owned(), value);
    // }

    // // deprecate
    // fn to_values(&self) -> (Vec<u8>, u32) {
    //     (self.transaction_hash.clone(), self.output_index)
    // }
    //
    pub fn to_input(&self) -> Input {
        let mut input = Input::default();
        input.utxo_id =  self.utxo_id.clone();
        input.output = self.output.clone();
        input
    }

    // pub fn address_index(&self) -> u32 {
    //     return self.to_values().1;
    // }
    //
    // pub fn transaction_hash(&self) -> Vec<u8> {
    //     return self.to_values().0;
    // }
    //
    // fn weights_to_bytes(weights: &Vec<f64>) -> Vec<u8> {
    //     let mut bytes: Vec<u8> = Vec::new();
    //     for weights in weights.iter() {
    //         bytes.extend_from_slice(&weights.to_le_bytes());
    //     }
    //     return bytes;
    // }

    pub fn from_output_new(
        output: &Output,
        transaction_hash: &Hash,
        output_index: i64,
        time: i64,
    ) -> UtxoEntry {
        return UtxoEntry {
            utxo_id: Some(UtxoId::new(transaction_hash, output_index)),
            output: Some(output.clone()),
            time,
        };
    }

    pub fn from_transaction(transaction: &Transaction, time: i64) -> Vec<UtxoEntry> {
        let map = transaction
            .outputs
            .iter()
            .enumerate()
            .map(|(i, output)| Self::from_output_new(output, &transaction.hash_or(), i as i64, time))
            .collect();
        return map;
    }

    // pub fn ids_from_transaction_outputs(transaction: &Transaction) -> Vec<Vec<u8>> {
    //     return transaction
    //         .outputs
    //         .iter()
    //         .enumerate()
    //         .map(|(i, _output)| {
    //             UtxoEntry::id_from_fixed(&FixedIdConvert::from_values(
    //                 &transaction.hash(),
    //                 i as u32,
    //             ))
    //         })
    //         .collect();
    // }

    // pub fn ids_from_transaction_inputs(transaction: &Transaction) -> Vec<Vec<u8>> {
    //     return transaction
    //         .inputs
    //         .iter()
    //         .map(|input| {
    //             UtxoEntry::id_from_values(&input.transaction_hash, &u32_to_vec(input.output_index))
    //         })
    //         .collect();
    // }
}
