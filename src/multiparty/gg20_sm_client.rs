use std::convert::TryInto;

use anyhow::{Context, Result};
use futures::{Sink, Stream, StreamExt, TryStreamExt};
use log::info;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
// use structopt::StructOpt;

use round_based::Msg;
use redgold_keys::request_support::RequestSupport;
use redgold_schema::{EasyJson, RgResult};
use redgold_schema::structs::Request;
use crate::core::relay::Relay;
use crate::node_config::NodeConfig;
use crate::schema::structs::MultipartyAuthenticationRequest;

pub async fn join_computation<M>(
    address: surf::Url,
    room_id: &str,
    relay: &Relay
) -> Result<(
    u16,
    impl Stream<Item = Result<Msg<M>>>,
    impl Sink<Msg<M>, Error = anyhow::Error>,
)>
    where
        M: Serialize + DeserializeOwned,
{
    let client = SmClient::new(address, room_id, relay).context("construct SmClient")?;

    // Construct channel of incoming messages
    let incoming = client
        .subscribe()
        .await
        .context("subscribe")?
        .and_then(|msg| async move {
            serde_json::from_str::<Msg<M>>(&msg).context("deserialize message")
        });

    // Obtain party index
    let index = client.issue_index().await.context("issue an index")?;
    info!("Multiparty join computation issued index: {}", index);
    // Ignore incoming messages addressed to someone else
    let incoming = incoming.try_filter(move |msg| {
        futures::future::ready(
            msg.sender != index && (msg.receiver.is_none() || msg.receiver == Some(index)),
        )
    });

    // Construct channel of outgoing messages
    let outgoing = futures::sink::unfold(client, |client, message: Msg<M>| async move {
        let serialized = serde_json::to_string(&message).context("serialize message")?;
        client
            .broadcast(&serialized)
            .await
            .context("broadcast message")?;
        Ok::<_, anyhow::Error>(client)
    });

    Ok((index, incoming, outgoing))
}

pub struct SmClient {
    http_client: surf::Client,
    relay: Relay,
    room_id: String
}

impl SmClient {
    pub fn new(address: surf::Url, room_id: &str, relay: &Relay) -> Result<Self> {
        let config = surf::Config::new()
            .set_base_url(address.join(&format!("rooms/{}/", room_id))?)
            .set_timeout(None);
        Ok(Self {
            http_client: config.try_into()?,
            relay: relay.clone(),
            room_id: room_id.to_string().clone(),
        })
    }

    pub async fn request(&self, message: Option<String>) -> Request {
        let mut req = Request::empty();
        let mut mpa = MultipartyAuthenticationRequest::default();
        mpa.message = message;
        mpa.room_id = self.room_id.clone();
        req.multiparty_authentication_request = Some(mpa);
        req = self.relay.sign_request(&mut req).await.expect("Bad signing request in SM client");
        let result = req.verify_auth();
        if result.is_err() {
            panic!("Wtf")
        }
        result.expect("Immediate verification failure");
        req
    }

    pub async fn issue_index(&self) -> Result<u16> {
        let req = self.request(None).await;
        let response = self
            .http_client
            .post("issue_unique_idx")
            .body(req.json_or())
            .recv_json::<IssuedUniqueIdx>()
            .await
            .map_err(|e| e.into_inner())?;
        Ok(response.unique_idx)
    }

    // TODO: Add auth
    pub async fn broadcast(&self, message: &str) -> Result<()> {
        let req = self.request(Some(message.to_string())).await;
        self.http_client
            .post("broadcast")
            .body(req.json_or())
            .await
            .map_err(|e| e.into_inner())?;
        Ok(())
    }

    // TODO: Add auth
    pub async fn subscribe(&self) -> Result<impl Stream<Item = Result<String>>> {
        let response = self
            .http_client
            .get("subscribe")
            .header("auth", self.request(None).await.json_or())
            .await
            .map_err(|e| e.into_inner())?;
        let events = async_sse::decode(response);
        Ok(events.filter_map(|msg| async {
            match msg {
                Ok(async_sse::Event::Message(msg)) => Some(
                    String::from_utf8(msg.into_bytes())
                        .context("SSE message is not valid UTF-8 string"),
                ),
                Ok(_) => {
                    // ignore other types of events
                    None
                }
                Err(e) => Some(Err(e.into_inner())),
            }
        }))
    }
}

#[derive(Deserialize, Debug)]
struct IssuedUniqueIdx {
    unique_idx: u16,
}
//
// #[derive(StructOpt, Debug)]
// struct Cli {
//     #[structopt(short, long)]
//     address: surf::Url,
//     #[structopt(short, long)]
//     room: String,
//     #[structopt(subcommand)]
//     cmd: Cmd,
// }
//
// #[derive(StructOpt, Debug)]
// enum Cmd {
//     Subscribe,
//     Broadcast {
//         #[structopt(short, long)]
//         message: String,
//     },
//     IssueIdx,
// }

//
// pub async fn run_() -> Result<()> {
//     let args: Cli = Cli::from_args();
//     let client = SmClient::new(args.address, &args.room).context("create SmClient")?;
//     match args.cmd {
//         Cmd::Broadcast { message } => client
//             .broadcast(&message)
//             .await
//             .context("broadcast message")?,
//         Cmd::IssueIdx => {
//             let index = client.issue_index().await.context("issue index")?;
//             println!("Index: {}", index);
//         }
//         Cmd::Subscribe => {
//             let messages = client.subscribe().await.context("subsribe")?;
//             tokio::pin!(messages);
//             while let Some(message) = messages.next().await {
//                 println!("{:?}", message);
//             }
//         }
//     }
//     Ok(())
// }
