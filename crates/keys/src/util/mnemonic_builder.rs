use crate::util;
use crate::util::mnemonic_support::MnemonicSupport;
use redgold_schema::keys::words_pass::WordsPass;

// Legacy usage, deprecated but still supported
pub fn from_str_rounds(s: &str, additional_rounds: usize) -> String {
    let mut hash = util::dhash_str(s);

    for _ in 0..(additional_rounds + 10000) {
        hash = util::dhash(&hash);
    }
    // Old function
    // use bdk::bitcoin_wallet::mnemonic::Mnemonic;
    // let mnemonic = Mnemonic::new(&hash).unwrap();
    let m2 = WordsPass::from_bytes(&hash).expect("Failed to create WordsPass from bytes");
    // println!("Mnemonic: {}", mnemonic.to_string());
    // println!("Mnemonic2: {}", m2.words);
    // assert_eq!(mnemonic.to_string(), m2.words);

    return m2.words;
}

#[test]
fn compare_str_rounds() {
    let s = "test";
    let additional_rounds = 100;
    let mnemonic = from_str_rounds(s, additional_rounds);
    // let mnemonic2 = from_str_rounds(s, additional_rounds);
    // assert_eq!(mnemonic, mnemonic2);
}
