use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use bdk::bitcoin::secp256k1::{Message, Secp256k1, Signature};
use bdk::bitcoin::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use futures::{SinkExt, StreamExt, TryStreamExt};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::SignatureRecid;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::LocalKey;
// use structopt::StructOpt;

use curv::arithmetic::Converter;
use curv::BigInt;

use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::sign::{
    OfflineStage, SignManual,
};
use round_based::async_runtime::AsyncProtocol;
use round_based::Msg;
use redgold_schema::{bytes_data, error_info, structs};
use redgold_schema::structs::{ErrorInfo, Proof};
use redgold_keys::util::verify;
use crate::multiparty_gg20::gg20_keygen::external_address_to_surf_url;

use crate::multiparty_gg20::gg20_sm_client::join_computation;

// #[derive(Debug, StructOpt)]
// struct Cli {
//     #[structopt(short, long, default_value = "http://localhost:8000/")]
//     address: surf::Url,
//     #[structopt(short, long, default_value = "default-signing")]
//     room: String,
//     #[structopt(short, long)]
//     local_share: PathBuf,
//
//     #[structopt(short, long, use_delimiter(true))]
//     parties: Vec<u16>,
//     #[structopt(short, long)]
//     data_to_sign: String,
// }

use curv::elliptic::curves::Point;
//
// pub fn decode_local_share_public_key(local_share: String) {
//     use curv::elliptic::curves::{secp256_k1::Secp256k1, Scalar};
//     let local_share = json_from::<LocalKey<Secp256k1>>(&*local_share);
//     let vec: Vec<Point<Secp256k1>> = local_share.pk_vec;
//     //vec.bytes()
//
// }

// #[tokio::main]
async fn signing_original(
    address: surf::Url, room: &str, local_share: String, parties: Vec<u16>, data_to_sign: Vec<u8>,
    relay: Relay
) -> Result<SignatureRecid> {

    let local_share = serde_json::from_str(&local_share).context("parse local share")?;
    let number_of_parties = parties.len();

    // Ahh here we go.
    info!("Starting signing join computation offline for room {} on node {}", room, relay.node_config.short_id().expect(""));

    let (i, incoming, outgoing) =
        join_computation(address.clone(), &format!("{}-offline", room), &relay)
            .await
            .context("join offline computation")?;

    let incoming = incoming.fuse();
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let signing = OfflineStage::new(i, parties, local_share)?;
    let completed_offline_stage = AsyncProtocol::new(signing, incoming, outgoing)
        .run()
        .await
        .map_err(|e| anyhow!("protocol execution terminated with error: {}", e))?;

    info!("Starting signing join computation online for room {} on node {}", room, relay.node_config.short_id().expect(""));

    let (i, incoming, outgoing) = join_computation(
        address, &format!("{}-online", room), &relay)
        .await
        .context("join online computation")?;

    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let (signing, partial_signature) = SignManual::new(
        BigInt::from_bytes(&*data_to_sign),
        completed_offline_stage,
    )?;

    outgoing
        .send(Msg {
            sender: i,
            receiver: None,
            body: partial_signature,
        })
        .await?;

    let partial_signatures: Vec<_> = incoming
        .take(number_of_parties - 1)
        .map_ok(|msg| msg.body)
        .try_collect()
        .await?;
    let signature = signing
        .complete(&partial_signatures)
        .context("online stage failed")?;
    // let signature = serde_json::to_string(&signature).context("serialize signature")?;
    // println!("{}", signature);

    Ok(signature)
}
use curv::elliptic::curves::ECScalar;
use tracing::info;
use redgold_schema::helpers::easy_json::{json, json_from};
use crate::core::relay::Relay;
use redgold_schema::conf::node_config::NodeConfig;
use crate::schema::structs::RsvSignature;

pub async fn signing(
    external_address: String, port: u16, room: String, local_share: String, parties: Vec<u16>, data_to_sign: Vec<u8>, relay: Relay
) -> Result<Proof, ErrorInfo> {
    let url = external_address_to_surf_url(external_address, port)?;
    let sig = signing_original(url, &*room, local_share, parties, data_to_sign.clone(), relay)
        .await
        .map_err(|e| error_info(e.to_string()))?;
    let mut vec: Vec<u8> = vec![];
    let r = sig.r.as_raw().serialize().to_vec();
    let s = sig.s.as_raw().serialize().to_vec();
    vec.extend_from_slice(&r);
    vec.extend_from_slice(&s);
    // TODO: skip later
    let sig_secp = Signature::from_compact(&*vec).map_err(|e| error_info(e.to_string()))?;
    // TODO: verify() -- where is the public key?
    let comp = sig_secp.serialize_compact().to_vec();
    let mut sig_struct = structs::Signature::ecdsa(comp.clone());
    let recovery_id = sig.recid;
    // recovery_id.to_be_bytes()

    let rsv_sig = RsvSignature {
        r: bytes_data(r),
        s: bytes_data(s),
        v: Some(recovery_id as i64)
    };

    // TODO: Technically, we can drop the other signature here for simplicity when an appropriate verify is in place
    // For now this is fine.
    sig_struct.rsv = Some(rsv_sig);

    let rec_id = RecoveryId::from_i32(recovery_id as i32).map_err(|e| error_info(e.to_string()))?;
    let rec_sig = RecoverableSignature::from_compact(&*comp, rec_id).map_err(|e| error_info(e.to_string()))?;

    let s = Secp256k1::new();
    let data_bytes = data_to_sign.clone();
    let msg = Message::from_slice(&data_bytes).map_err(|e| error_info(e.to_string()))?;
    let recovered_pk = s.recover(&msg, &rec_sig).map_err(|e| error_info(e.to_string()))?;
    let public = structs::PublicKey::from_bytes_direct_ecdsa(recovered_pk.serialize().to_vec());
    let proof = Proof::from(public, sig_struct);
    Ok(proof)
}
