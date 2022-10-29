use multihash::{Code, MultihashDigest};

#[test]
fn example() {
    let hash = Code::Sha2_256.digest(b"my hash");
    println!("{:?}", hash.to_bytes());
}
