use std::collections::HashMap;
use std::str::FromStr;
use std::string::ToString;
use bdk::bitcoin::util::bip32::ExtendedPubKey;
use bdk::bitcoin::util::psbt::serialize::Serialize;
use itertools::{all, Itertools};
use redgold_keys::proof_support::{ProofSupport, PublicKeySupport};
use redgold_keys::TestConstants;
use redgold_keys::transaction_support::InputSupport;
use redgold_schema::{error_info, ErrorInfoContext, SafeOption, structs};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{AddressInfo, CurrencyAmount, ErrorInfo, Hash, Input, NetworkEnvironment, Output, Proof, Signature, Transaction, UtxoEntry, UtxoId};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_common_no_wasm::cmd::{run_cmd, run_cmd_safe};
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use crate::util::keys::{public_key_from_bytes, ToPublicKeyFromLib};
use crate::util::init_logger_once;

const MISSING_DEVICE: &str = "Failed to find a Trezor device";
const TREZORCTL: &str = "trezorctl";

pub fn trezor_cmd(args: Vec<&str>) -> Result<String, ErrorInfo> {
    // tracing::debug!("Running trezor cmd with args {:?}", args.clone());
    let res = run_cmd_safe(TREZORCTL, args)?;
    // tracing::debug!("Trezor command raw output: {:?}", res.clone());
    if res.0.contains(MISSING_DEVICE) {
        Err(error_info("Failed to find a Trezor device".to_string()))?;
    }
    Ok(res.0)
}

// Failed to read details
pub fn trezor_list_devices() -> Result<Option<String>, ErrorInfo> {
    let res = trezor_cmd(vec!["list"])?;
    let res = if res.is_empty() {
        None
    } else {
        let strr = res.split("\n")
            .filter(|l| {
                !(l.contains("Failed to read details") || l.trim().is_empty())
            })
            .next()
            .map(|l| l.to_string());
        strr
    };
    Ok(res)
}

#[ignore]
#[test]
fn test_list_devices() {
    let res = trezor_list_devices();
    println!("Trezor list devices: {:?}", res);
    assert!(res.is_ok());
}

pub fn parse_output(output: String) -> (HashMap<String, String>, Vec<(String, String)>) {
    let mut hm = HashMap::default();
    let mut vec = Vec::default();
    let lines = output.lines();
    for mut l in lines.map(|l| l.split(": ")) {
        if let Some(k) = l.next() {
            if let Some(v) = l.next() {
                let tuple = (k.to_string(), v.to_string());
                vec.push(tuple);
                hm.insert(k.to_string(), v.to_string());
            }
        }
    }
    (hm, vec)
}

pub fn run_trezor_cmd(args: Vec<&str>) -> Result<(HashMap<String, String>, Vec<(String, String)>), ErrorInfo> {
    let res = trezor_cmd(args)?;
    let map = parse_output(res);
    println!("Parsed output: {:?}", map.clone());
    Ok(map)
}


/*

trezorctl get_public_node -n "m/44'/0'/0'/0/0"

Please confirm action on your Trezor device.
node.depth: 5
node.fingerprint: <etc>
node.child_num: 0
node.chain_code: <etc>
node.public_key: <your_key_here>
xpub: <your_key_here>
 */

#[derive(Clone, Debug)]
pub struct PublicNodeResponse {
    node_depth: u32,
    node_fingerprint: String,
    node_child_num: u32,
    node_chain_code: String,
    node_public_key: String,
    pub(crate) xpub: String
}

impl PublicNodeResponse {
    pub fn public_key(&self) -> Result<structs::PublicKey, ErrorInfo> {
        let h = hex::decode(&self.node_public_key).error_info("Failed to decode public key")?;
        let public_key_parse_test = public_key_from_bytes(&h).error_info("Failed to parse public key")?;
        Ok(public_key_parse_test.to_struct_public_key())
    }
    pub fn xpub(&self) -> anyhow::Result<ExtendedPubKey, ErrorInfo> {
        ExtendedPubKey::from_str(&self.xpub).error_info("Failed to parse xpub")
    }
}

pub fn get_public_node(path: String) -> Result<PublicNodeResponse, ErrorInfo> {
    let (hm, vec) = run_trezor_cmd(vec!["get-public-node", "-n", &path])?;
    println!("keys: {:?}", hm.keys());
    //
    // let node_depth =
    //     vec.get(0).ok_or(
    //     // hm.get("node.depth").ok_or(
    //     Err(error_info("Failed to find node.depth output line".to_string()))?
    // )?.parse::<u32>().error_info("Failed to parse node.depth")?;
    //
    let node_public_key = vec.get(4).safe_get_msg("no public key")?.1.clone();
    let xpub = vec.get(5).safe_get_msg("no xpub")?.1.clone();
    //     vec.get("node.public_key").ok_or(
    //     // hm.get("node.public_key").ok_or(
    //     Err(error_info("Failed to find node.public_key output line".to_string()))?
    // )?.clone();
    //
    // let node_fingerprint = hm.get("node.fingerprint").ok_or(
    //     Err(error_info("Failed to find node.fingerprint output line".to_string()))?
    // )?.clone();
    // let node_child_num = hm.get("node.child_num").ok_or(
    //     Err(error_info("Failed to find node.child_num output line".to_string()))?
    // )?.parse::<u32>().error_info("Failed to parse node.child_num")?;
    // let node_chain_code = hm.get("node.chain_code").ok_or(
    //     Err(error_info("Failed to find node.chain_code output line".to_string()))?
    // )?.clone();
    //
    // let xpub = hm.get("xpub").ok_or(
    //     Err(error_info("Failed to find xpub output line".to_string()))?
    // )?.clone();
    Ok(PublicNodeResponse{
        node_depth: 0,
        node_fingerprint: "".to_string(),
        node_child_num: 0,
        node_chain_code: "".to_string(),
        node_public_key,
        xpub,
    })
}



/// Message signing

#[derive(Clone)]
pub struct SignMessageResponse {
    // message: String,
    address: String,
    signature: Vec<u8>,
}

impl SignMessageResponse {
    pub fn recovery_id(&self) -> i32 {
        self.signature[0] as i32 - 31
    }
    pub fn signature_vec(&self) -> Vec<u8> {
        self.signature[1..].to_vec()
    }
    pub fn signature(&self) -> Signature {
        Signature::hardware(self.signature_vec())
    }
}

/*

trezorctl sign-message -n "m/44'/0'/0'/0/0" "ahoy"

Please confirm action on your Trezor device.
message: ahoy
address: <some_address>
signature: <some_signature>
 */
pub fn sign_message(path: String, input_message: String) -> Result<SignMessageResponse, ErrorInfo> {
    let res = trezor_cmd(vec!["sign-message", "-n", &path, &input_message])?;
    let (_, vec) = parse_output(res);

    let _message = vec.get(0).safe_get_msg("no message")?.1.clone();
    let address = vec.get(1).safe_get_msg("no address")?.1.clone();
    let sig = vec.get(2).safe_get_msg("no signature")?.1.clone();
    // Strange error here happens when attempting to use hashmap keys, they match but it doesn't work
    // May be a string encoding issue?
    // Either way order preserving vec works for now.
    // let sig = hm.get("signature").ok_or(
    //     Err(error_info("Failed to find signature output line".to_string()))?
    // )?;
    // let message = hm.get("message").ok_or(
    //     Err(error_info("Failed to find message output line".to_string()))?
    // )?.clone();
    // let address = hm.get("address").ok_or(
    //     Err(error_info("Failed to find address output line".to_string()))?
    // )?.clone();
    let signature = base64::decode(sig).error_info("Failed to decode base64 signature")?;
    tracing::debug!("signature length: {:?}", signature.len());
    // let message = base64::decode(message).error_info("Failed to decode base64 message")?;
    Ok(SignMessageResponse{
        // message,
        address,
        signature,
    })
}



pub const DEFAULT_ACCOUNT_NUM: u32 = 50;

pub fn default_pubkey_path() -> String {
    format!("m/44'/0'/{}'/0/0", DEFAULT_ACCOUNT_NUM)
}

pub fn default_xpub_path() -> String {
    format!("m/44'/0'/{}'", DEFAULT_ACCOUNT_NUM)
}

pub fn default_pubkey() -> Result<structs::PublicKey, ErrorInfo> {
    get_public_node(default_pubkey_path())?.public_key()
}

pub fn default_xpub() -> Result<String, ErrorInfo> {
    Ok(get_public_node(default_pubkey_path())?.xpub)
}

/// This returns a hardened xpub so it doesn't expose other accounts
pub fn standard_xpub_path(account_num: u32, non_standard_coin_type: Option<String>) -> String {
    let coin_type = non_standard_coin_type.unwrap_or("0".to_string()).to_string();
    return format!("m/44'/{}'/{}'", coin_type, account_num.to_string()).to_string();
}

/// Primary function used for finding getting an extended public key
/// This supplies us with the information required to construct all public keys
/// for a given account
/// This is one of two dependencies on the trezor device and the primary interface for external
/// usage
pub fn get_standard_xpub(account_num: u32, non_standard_coin_type: Option<String>) -> Result<String, ErrorInfo> {
    Ok(get_public_node(standard_xpub_path(account_num, non_standard_coin_type))?.xpub)
}

pub fn get_standard_public_key(
    account_num: u32,
    non_standard_coin_type: Option<String>,
    change: u32,
    index: u32
) -> Result<structs::PublicKey, ErrorInfo> {
    let path = trezor_bitcoin_standard_path(account_num, non_standard_coin_type, change, index);
    get_public_node(path)?.public_key()
}

pub fn trezor_bitcoin_standard_path(account_num: u32, non_standard_coin_type: Option<String>, change: u32, index: u32) -> String {
    format!("{}/{}/{}",
            standard_xpub_path(account_num, non_standard_coin_type),
            change.to_string(),
            index.to_string()
    )
}

//
// pub fn prepare_message(tx: Transact) -> String {
//     format!("{}{}", SIGN_MESSAGE_HEADER, msg)
// }


/// PublicKey here should ideally be derived from an xpub requested earlier in the process
/// And needs to match the supplied path
pub fn trezor_proof(hash: &Hash, public: structs::PublicKey, path: String) -> Result<Proof, ErrorInfo> {
    let msg = redgold_keys::util::bitcoin_message_signer::message_from_hash(hash);
    let signature = sign_message(path, msg)?;
    let sig = signature.signature();
    let proof = Proof::from(public, sig);
    proof.verify(hash)?;
    Ok(proof)
}



pub async fn sign_transaction(transaction: &mut Transaction, public: structs::PublicKey, path: String)
    -> Result<Transaction, ErrorInfo> {
    let hash: Hash = transaction.hash_or();
    let ser_transaction = transaction.json_or();
    let all_addrs = public.to_all_addresses()?;

    for i in transaction.inputs.iter_mut() {

        let enriched_output = i.output
            .safe_get_msg("Failed to get output on transaction input")
            .with_detail("input", i.json_or())
            .with_detail("transaction", ser_transaction.clone())?;
        let output_enriched_address = enriched_output.address
            .safe_get_msg("Failed to get address")?;
        if all_addrs.contains(&output_enriched_address) {
            let proof = trezor_proof(&hash, public.clone(), path.clone())?;
            let proof1 = proof.clone();
            i.proof.push(proof1);
            i.verify_proof(&output_enriched_address, &hash)?;
        }
    }
    Ok(transaction.clone())
}

// Needs to match trezor expected format
pub async fn sign_bitcoin_transaction(json_str: String) -> Result<String, ErrorInfo> {
    trezor_cmd(vec!["btc", "sign-tx", &*json_str])
}

pub async fn sign_input(i: &mut Input, public: &structs::PublicKey, path: String, hash: &Hash)
    -> Result<Input, ErrorInfo> {
    let proof = trezor_proof(&hash, public.clone(), path.clone())?;
    let addr = public.address()?;
    let proof1 = proof.clone();
    i.proof.push(proof1);
    i.verify_proof(&addr, hash)?;
    Ok(i.clone())
}


#[ignore]
#[tokio::test]
async fn debug_sign_tx () {
    init_logger_once();
    let _tc = TestConstants::new();
    let pk = default_pubkey().expect("pk");
    let address = pk.address().expect("a");
    let _addr = address.clone().hex();
    let hash = Hash::from_string_calculate("test");
    let utxo = UtxoEntry{
        utxo_id: Some(UtxoId::new(&hash, 0)),
        output: Some(Output::new(&address, 100)),
        time: 0,
    };
    let ai = AddressInfo{
        address: None,
        utxo_entries: vec![utxo],
        balance: 0,
        recent_transactions: vec![]
    };
    let nc = NodeConfig::default();
    let mut tb = TransactionBuilder::new(&nc);
    tb.with_address_info(ai);
    let destination = address.clone();
    let amount = CurrencyAmount::from(5);
    tb.with_output(&destination, &amount);
    let res = tb.build().expect("tx");
    sign_transaction(&mut res.clone(), pk, default_pubkey_path())
        .await.expect("sign");
}


/// Debug / local testing and verification

#[ignore]
#[test]
pub fn trezor_test() {
    //
    // let cursor = HDPathCursor::from( 0, 0, 0, 0);
    // let node = get_public_node(cursor.to_string()).unwrap();
    // // let res = node.public_key();
    // // let xpub = node.xpub().unwrap();
    // let res = node.public_key().unwrap();
    //
    // println!("Got public key {}", res.hex().expect(""));
    // let msg = "ahoy";
    // let signature = sign_message(cursor.to_string(), msg.to_string()).unwrap();
    // let _sig = Signature::ecdsa(signature.signature_vec());
    // let proof = Proof::from(res, sig);
    // proof.verify(&vec).unwrap();
    // println!("Verified proof");

}

/*
Is this accurate??
https://bitcoin.stackexchange.com/questions/38351/ecdsa-v-r-s-what-is-v/38909
31 compressed public key, y-parity 0, magnitude of x lower than the curve order
32 compressed public key, y-parity 1, magnitude of x lower than the curve order
33 compressed public key, y-parity 0, magnitude of x greater than the curve order
34 compressed public key, y-parity 1, magnitude of x greater than the curve order


    if not utils.BITCOIN_ONLY and coin.decred:
        h = utils.HashWriter(blake256())
    else:
        h = utils.HashWriter(sha256())
    if not coin.signed_message_header:
        raise wire.DataError("Empty message header not allowed.")
    write_compact_size(h, len(coin.signed_message_header))
    h.extend(coin.signed_message_header.encode())
    write_compact_size(h, len(message))
    h.extend(message)
    ret = h.get_digest()
    if coin.sign_hash_double:
        ret = sha256(ret).digest()
    return ret


    message = msg.message
    address_n = msg.address_n
    script_type = msg.script_type or InputScriptType.SPENDADDRESS

    await validate_path(
        ctx, keychain, address_n, validate_path_against_script_type(coin, msg)
    )

    node = keychain.derive(address_n)
    address = get_address(script_type, coin, node)
    await confirm_signverify(
        ctx,
        coin.coin_shortcut,
        decode_message(message),
        address_short(coin, address),
        verify=False,
    )

    seckey = node.private_key()

    digest = message_digest(coin, message)
    signature = secp256k1.sign(seckey, digest)

    if script_type == InputScriptType.SPENDADDRESS:
        script_type_info = 0
    elif script_type == InputScriptType.SPENDP2SHWITNESS:
        script_type_info = 4
    elif script_type == InputScriptType.SPENDWITNESS:
        script_type_info = 8
    else:
        raise wire.ProcessError("Unsupported script type")

    # Add script type information to the recovery byte.
    if script_type_info != 0 and not msg.no_script_type:
        signature = bytes([signature[0] + script_type_info]) + signature[1:]

    return MessageSignature(address=address, signature=signature)

/// Hash message for signature using Bitcoin's message signing format
pub fn signed_msg_hash(msg: &str) -> sha256d::Hash {
    sha256d::Hash::hash(
        &[
            MSG_SIGN_PREFIX,
            &encode::serialize(&encode::VarInt(msg.len() as u64)),
            msg.as_bytes(),
        ]
        .concat(),
    )
}



 */

