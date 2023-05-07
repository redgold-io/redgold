use std::collections::HashMap;
use itertools::Itertools;
use redgold_schema::{error_info, ErrorInfoContext};
use redgold_schema::structs::{ErrorInfo, Signature};
use crate::util::cmd::run_cmd;
use crate::util::keys::public_key_from_bytes;


const MISSING_DEVICE: &str = "Failed to find a Trezor device";
const TREZORCTL: &str = "trezorctl";





#[test]
pub fn test_trezor_decode() {
    let h = hex::decode("028bc0d6dcf1f961214cbe49d013f64f77bc1c5802684add974546308bcf7210d1").expect("dec");
    public_key_from_bytes(&h).expect("bytes");
}
/*

trezorctl get_public_node -n "m/44'/0'/0'/0/0"

Please confirm action on your Trezor device.
node.depth: 5
node.fingerprint: 609425c1
node.child_num: 0
node.chain_code: 369311ac3556567434ba2d463f6204742303b2505b93ed6a3bb54cc6cf6f1c17
node.public_key: 028bc0d6dcf1f961214cbe49d013f64f77bc1c5802684add974546308bcf7210d1
xpub: xpub6G8DGdDBqgQHumfdCoyMXXVU9PeoyxrRMk6ERUHzKPtQtKU4GSAD9roZtMR9YiesmqYeXXyXgZxr4jrcXkX5qA9sUxRyZQK9ZLE4Au5LUAY
 */

pub struct PublicNodeResponse {
    node_depth: u32,
    node_fingerprint: String,
    node_child_num: u32,
    node_chain_code: String,
    node_public_key: String,
    xpub: String
}

pub fn get_public_node(path: String) {

}

pub fn trezor_cmd(args: Vec<&str>) -> Result<String, ErrorInfo> {
    let res = run_cmd(TREZORCTL, args);
    if res.0.contains(MISSING_DEVICE) {
        Err(error_info("Failed to find a Trezor device".to_string()))?;
    }
    Ok(res.0)
}

pub fn parse_output(output: String) -> HashMap<String, String> {
    let mut hm = HashMap::default();
    let lines = output.lines().dropping(1);
    for mut l in lines.map(|l| l.split(": ")) {
        if let Some(k) = l.next() {
            if let Some(v) = l.next() {
                hm.insert(k.to_string(), v.to_string());
            }
        }
    }
    hm
}

#[derive(Clone)]
pub struct SignMessageResponse {
    message: String,
    address: String,
    signature: Signature,
}


/*

trezorctl sign-message -n "m/44'/0'/0'/0/0" "ahoy"

Please confirm action on your Trezor device.
message: ahoy
address: 1NhvEArmzNMRXyoppD6WiE5wztTbb85rb4
signature: IP1/3VEYXpad93ssZ8UU3qG3jpqyTcV4JHp299H/TsGzDABl/SnX8NICx2zWHw1TsDUc/REtfyb7lcb1lvhd9Kc=
 */
pub fn sign_message(path: String, input_message: String) -> Result<SignMessageResponse, ErrorInfo> {
    let res = trezor_cmd(vec!["sign-message", "-n", &path, &input_message])?;
    let hm = parse_output(res);
    let sig = hm.get("signature").ok_or(
        Err(error_info("Failed to find signature output line".to_string()))?
    )?;
    let message = hm.get("message").ok_or(
        Err(error_info("Failed to find message output line".to_string()))?
    )?.clone();
    let address = hm.get("address").ok_or(
        Err(error_info("Failed to find address output line".to_string()))?
    )?.clone();
    let b64 = base64::decode(sig).error_info("Failed to decode base64 signature")?;
    let signature = Signature::ecdsa(b64);
    Ok(SignMessageResponse{
        message,
        address,
        signature,
    })
}

#[test]
pub fn trezor_test() {
    /*

trezorctl get-address -n "m/49'/0'/0'/0/0" -t "p2shsegwit" -d
trezorctl sign-message -n "m/44'/0'/0'/0/0" "ahoy"

     */
    // let res = run_cmd(
    //     "trezorctl",
    //     vec!["sign-message", "-n", "m/501201'/0'/0'/0/0", "ahoy"]
    // );
    // // res: ("Failed to find a Trezor device.\n", "")
    // println!("res: {:?}", res);

}