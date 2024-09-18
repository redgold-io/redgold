use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use std::path::PathBuf;
use log::info;
// use structopt::StructOpt;

use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::async_runtime::AsyncProtocol;
use redgold_schema::error_info;
use redgold_schema::structs::ErrorInfo;
use crate::core::relay::Relay;
use redgold_schema::conf::node_config::NodeConfig;

use super::gg20_sm_client::join_computation;
//
// #[derive(Debug, StructOpt)]
// struct Cli {
//     #[structopt(short, long, default_value = "http://localhost:8000/")]
//     address: surf::Url,
//     #[structopt(short, long, default_value = "default-keygen")]
//     room: String,
//     #[structopt(short, long)]
//     output: PathBuf,
//
//     #[structopt(short, long)]
//     index: u16,
//     #[structopt(short, long)]
//     threshold: u16,
//     #[structopt(short, long)]
//     number_of_parties: u16,
// }

async fn keygen_original(
    address: surf::Url,
    room: &str,
    index: u16,
    threshold: u16,
    number_of_parties: u16,
    relay: &Relay
) -> Result<String> {

    // info!("Starting join computation for room {} on node {} index: {}", room, relay.node_config.short_id().expect(""), index);
    let (_i, incoming, outgoing) =
        join_computation(address, room, relay)
        .await
        .context("join computation")?;

    // println!("Finished join computation");

    let incoming = incoming.fuse();
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let keygen = Keygen::new(index, threshold, number_of_parties)?;
    let output = AsyncProtocol::new(keygen, incoming, outgoing)
        .run()
        .await
        .map_err(|e| anyhow!("protocol execution terminated with error: {}", e))?;
    let output = serde_json::to_string_pretty(&output).context("serialize output")?;
    Ok(output)
}

pub fn external_address_to_surf_url(external_address: String, port: u16) -> Result<surf::Url, ErrorInfo> {
    let address = format!("http://{}:{}/", external_address, port.to_string());
    let url = surf::Url::parse(&*address).map_err(|e| error_info(e.to_string()))?;
    Ok(url)
}

pub async fn keygen(
    external_address: String,
    port: u16,
    room: String,
    index: u16,
    threshold: u16,
    number_of_parties: u16,
    relay: Relay
) -> Result<String, ErrorInfo>  {
    let url = external_address_to_surf_url(external_address, port)?;
    keygen_original(url, &*room, index, threshold, number_of_parties, &relay)
        .await
        .map_err(|e| error_info(e.to_string()))

}
