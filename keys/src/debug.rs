use redgold_schema::structs::Hash;
use crate::util::mnemonic_support::WordsPass;

#[test]
fn xor_test() {
    let wp = WordsPass::generate().expect("g");
    let pk = wp.default_public_key().expect("d");
    // let b = pk.bytes().expect("b");
    // println!("b: {}", b.len());
    // println!("b: {}", Hash::digest("asdf".as_bytes().to_vec()).vec().len());
}