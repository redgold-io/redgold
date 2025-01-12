use std::time::Instant;
use sha3::Digest;
use crate::{bytes_data, RgResult, SafeOption};
use crate::proto_serde::ProtoSerde;
use crate::structs::{Hash, PoWProof, PoWProofType, Transaction};


fn check_difficulty_bytes(vec: &Vec<u8>, leading_zeros_bytes: usize) -> bool {
    vec.len() >= leading_zeros_bytes &&
    vec.iter().take(leading_zeros_bytes).all(|&b| b == 0)
}

fn check_difficulty_bits(vec: &Vec<u8>, leading_zero_bits: usize) -> bool {
    let mut count = 0;
    for &byte in vec.iter() {
        if byte == 0 {
            // If the byte is 0, all its bits are zero.
            count += 8; // 8 bits in a byte
        } else {
            // If the byte is not 0, count the leading zeros and break.
            count += byte.leading_zeros() as usize; // Count leading zeros in this byte
            break;
        }
    }
    count > leading_zero_bits
}



impl PoWProof {
    pub fn from_hash_sha3_256(hash: &Hash, difficulty: usize) -> RgResult<PoWProof> {
        let mut proof = PoWProof::default();
        proof.proof_type = PoWProofType::Sha3256Nonce as i32;
        let mut nonce: i64 = 0;
        loop {
            proof.index_counter = bytes_data(nonce.to_be_bytes().to_vec());
            if proof.verify(hash, difficulty)? {
                break
            }
            nonce += 1;
        }
        // While true, increment the nonce and check if the hash meets the difficulty target

        Ok(proof)
    }

    pub fn merged_bytes(&self, hash: &Hash) -> RgResult<Vec<u8>> {
        let hash_bytes = hash.proto_serialize();
        let nonce = self.index_counter.safe_get()?.value.clone();
        let mut merged = hash_bytes.clone();
        merged.extend_from_slice(&*nonce);
        Ok(merged)
    }

    pub fn merged_digest_hash(&self, hash: &Hash) -> RgResult<Hash> {
        let merged = self.merged_bytes(hash)?;
        Ok(Hash::digest(merged))
    }

    pub fn verify(&self, hash: &Hash, difficulty: usize) -> RgResult<bool> {
        Ok(check_difficulty_bytes(&self.merged_digest_hash(&hash)?.raw_bytes()?, difficulty))
    }

    pub fn merged_hex(&self, hash: &Hash) -> RgResult<String> {
        Ok(hex::encode(self.merged_digest_hash(hash)?.raw_bytes()?))
    }

    pub fn nonce_int(&self) -> RgResult<i64> {
        Ok(i64::from_be_bytes(self.index_counter.safe_get()?.value.clone().as_slice().try_into().expect("nonce")))
    }
}


#[test]
fn test_pow() {
    // This means we want 1 u8 leading zeroes
    let difficulty = 1;

    let mut elapsed_all = vec![];

    for i in 0..30 {
        let now = Instant::now();
        let string = format!("hello{}", i);
        let hash = Hash::from_string_calculate(&*string);
        let proof = PoWProof::from_hash_sha3_256(&hash, difficulty).expect("proof");
        proof.verify(&hash, difficulty).expect("verify");
        println!("hash: {:?}", hash.hex());
        println!("merged hex bytes: {:?}", hex::encode(proof.merged_bytes(&hash).expect("")));
        println!("Proof merged hash: {:?}", proof.merged_hex(&hash).expect("merged_hex"));
        println!("Proof merged bytes: {:?}", proof.merged_digest_hash(&hash).expect("merged_hex").vec());
        println!("Proof nonce: {:?}", proof.nonce_int().expect("nonce_int"));
        let elapsed = now.elapsed();
        elapsed_all.push(elapsed.as_millis());
        println!("Elapsed: {:?}", elapsed);
    }
    let avg = elapsed_all.iter().sum::<u128>() / elapsed_all.len() as u128;
    println!("Average: {:?}", avg);

}

pub trait TransactionPowValidate {
    fn pow_validate(&self) -> RgResult<bool>;
}

impl TransactionPowValidate for Transaction {
    fn pow_validate(&self) -> RgResult<bool> {
        let options = self.options.safe_get_msg("Missing tx options in pow validate")?;
        let proof = options
            .pow_proof.safe_get_msg("Missing pow proof")?;
        proof.verify(&self.signable_hash(), 1)
    }
}