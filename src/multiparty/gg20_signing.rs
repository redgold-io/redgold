use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use bitcoin::secp256k1::recovery::{RecoverableSignature, RecoveryId};
use bitcoin::secp256k1::{Message, Secp256k1, Signature};
use futures::{SinkExt, StreamExt, TryStreamExt};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::SignatureRecid;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::LocalKey;
use structopt::StructOpt;

use curv::arithmetic::Converter;
use curv::BigInt;

use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::sign::{
    OfflineStage, SignManual,
};
use round_based::async_runtime::AsyncProtocol;
use round_based::Msg;
use redgold_schema::{error_info, json, json_from, structs};
use redgold_schema::structs::{ErrorInfo, Proof};
use redgold_schema::util::verify;
use crate::multiparty::gg20_keygen::external_address_to_surf_url;

use crate::multiparty::gg20_sm_client::join_computation;

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
async fn signing_original(address: surf::Url, room: &str, local_share: String, parties: Vec<u16>, data_to_sign: Vec<u8>
) -> Result<SignatureRecid> {

    let local_share = serde_json::from_str(&local_share).context("parse local share")?;
    let number_of_parties = parties.len();

    let (i, incoming, outgoing) =
        join_computation(address.clone(), &format!("{}-offline", room))
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

    let (i, incoming, outgoing) = join_computation(
        address, &format!("{}-online", room))
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
pub async fn signing(
    external_address: String, port: u16, room: String, local_share: String, parties: Vec<u16>, data_to_sign: Vec<u8>
) -> Result<Proof, ErrorInfo> {
    let url = external_address_to_surf_url(external_address, port)?;
    let sig = signing_original(url, &*room, local_share, parties, data_to_sign.clone())
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
    let sig_struct = structs::Signature::ecdsa(comp.clone());
    let recovery_id = sig.recid;
    let rec_id = RecoveryId::from_i32(recovery_id as i32).map_err(|e| error_info(e.to_string()))?;
    let rec_sig = RecoverableSignature::from_compact(&*comp, rec_id).map_err(|e| error_info(e.to_string()))?;

    let mut s = Secp256k1::new();
    let data_bytes = data_to_sign.clone();
    let msg = Message::from_slice(&data_bytes).map_err(|e| error_info(e.to_string()))?;
    let recovered_pk = s.recover(&msg, &rec_sig).map_err(|e| error_info(e.to_string()))?;
    let public = structs::PublicKey::from_bytes(recovered_pk.serialize().to_vec());
    let proof = Proof::from(public, sig_struct);
    Ok(proof)
}
