use crate::util;
use bitcoin_wallet::mnemonic::Mnemonic;
use crypto::digest::Digest;
use crypto::sha2::Sha512;

// fix mnemonic length
// allow for optional passphrase by doing
// seed -> words + passphrase -> seed2

pub fn from_str_rounds(s: &str, additional_rounds: usize) -> Mnemonic {
    let mut hash = util::dhash_str(s);

    for _ in 0..(additional_rounds + 10000) {
        hash = util::dhash(&hash);
    }

    let mnemonic = Mnemonic::new(&hash).unwrap();

    return mnemonic;
}

pub struct HashDerivedMnemonic {
    pub hash: [u8; 32],
    pub mnemonic: Mnemonic,
}

impl HashDerivedMnemonic {
    pub fn checksum(&self) -> Vec<u8> {
        let merged = util::merge_hash(self.hash, util::dhash_str("Redgold_checksum_salt"));
        return util::checksum(&*merged.to_vec());
    }
    pub fn offset_derivation_path_salted_hash(&self, path: &str) -> HashDerivedMnemonic {
        let merged = util::merge_hash(
            self.hash,
            util::dhash_str(&*("Redgold_offset_salt/".to_owned() + path)),
        );
        HashDerivedMnemonic {
            hash: merged,
            mnemonic: Mnemonic::new(&merged).unwrap(),
        }
    }
}

pub fn from_str_rounds_preserve_hash(s: &str, additional_rounds: usize) -> HashDerivedMnemonic {
    let mut hash = util::dhash_str(s);

    for _ in 0..(additional_rounds + 10000) {
        hash = util::dhash(&hash);
    }

    let mnemonic = Mnemonic::new(&hash).unwrap();

    return HashDerivedMnemonic { hash, mnemonic };
}

pub fn from_str(s: &str) -> Mnemonic {
    let mut hash = [0u8; 64];
    let mut sha2 = Sha512::new();
    sha2.input_str(s);
    sha2.result(&mut hash);

    //println!("hash {:?}", hash.to_vec());

    let mnemonic = Mnemonic::new(&hash).unwrap();
    //println!("words: {}", mnemonic.to_string());

    return mnemonic;

    // Hilarious failures below.
    //
    // for b in 0..24 {
    //     let left = hash[b*2];
    //     let right = hash[b*2+1];
    //     let merged = u16::from_le_bytes([left, right]);
    //     let divmod = merged % 2048;
    //     println!("left {:?} right {:?} merged {:?} divmod {:?}", left, right, merged, divmod);
    // }
    //
    //
    // let bytes = [1, 2];
    // u16::from_le_bytes(bytes);
    //
    // let exact = hash.chunks_exact(2);
    //
    // exact.for_each(| c| -> c.)
    //
    // let res = exact.map(
    //     |c| u16::from_le_bytes([c.get(0).unwrap(), c.get(1).unwrap()])
    // );
    // println!("u16 hash {:?}", res.to_vec());
}

#[test]
fn test_hash() {
    assert_eq!(
    "divorce success kingdom guide abuse tuna citizen myself close scale quick music steel metal gorilla genuine invest mosquito involve group chef behind cage wait dance silver quantum lady dust oven fence primary response advice entire canal bring soap source fame wash consider right glue foil control egg blur",
    from_str("asdf").to_string()
    );
}
