use crate::core::internal_message::{Channel, new_channel, SendErrorInfo};
use crate::core::relay::{MultipartyRequestResponse, MultipartyRoomInternalSubscribe, Relay};

fn debug(relay: Relay) {

    let subscribe = new_channel::<MultipartyRoomInternalSubscribe>();
    // relay.multiparty

}
use std::convert::TryInto;

use anyhow::{Context, Result};
use futures::{Sink, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use structopt::StructOpt;
use redgold_schema::{json, json_from, SafeOption, structs};
use redgold_schema::structs::{ErrorInfo, InitiateMultipartyKeygenRequest, InitiateMultipartyKeygenResponse, MultipartyBroadcast, MultipartyIssueUniqueIndex, MultipartySubscribe, MultipartySubscribeEvent, MultipartyThresholdRequest, NodeMetadata, PublicKey, Request, Response};

use round_based::Msg;


pub async fn join_computation<'a, M>(
    client: &'a SmClient,
    receiver: &'a flume::Receiver<MultipartySubscribeEvent>,
) -> Result<(
    u16,
    impl Stream<Item = Result<Msg<M>, ErrorInfo>> + 'a,
    impl Sink<Msg<M>, Error = ErrorInfo> + 'a,
), ErrorInfo>
    where
        M: Serialize + DeserializeOwned + 'a,
{

    client.clone().subscribe().await?;
    let incoming = SmClient::process_subscription(&receiver)?
        .and_then(|msg| async move {
            json_from::<Msg<M>>(&msg)
        });

    // Obtain party index
    let index = client.issue_index().await?;

    // Ignore incoming messages addressed to someone else
    let incoming = incoming.try_filter(move |msg| {
        futures::future::ready(
            msg.sender != index && (msg.receiver.is_none() || msg.receiver == Some(index)),
        )
    });

    // Construct channel of outgoing messages
    let outgoing = futures::sink::unfold(client, |client, message: Msg<M>| async move {
        let serialized = json(&message)?;
        client
            .broadcast(&serialized)
            .await?;
        Ok::<_, ErrorInfo>(client)
    });

    Ok((index, incoming, outgoing))
}

#[derive(Clone)]
pub struct SmClient {
    relay: Relay,
    room_id: String,
    node_public: PublicKey,
    pub sub_channel: Channel::<MultipartySubscribeEvent>
}

impl SmClient {
    pub fn new(relay: Relay, node_public: PublicKey, room_id: String) -> Self {
        Self {
            relay,
            room_id,
            node_public,
            sub_channel: new_channel::<MultipartySubscribeEvent>(),
        }
    }

    // this going to be used by self so we need to make sure we can route requests to ourself
    pub async fn message(&self, request: Request) -> Result<Response, ErrorInfo> {
        self.relay.send_message_sync(request, self.node_public.clone(), None).await
    }

    pub async fn issue_index(&self) -> Result<u16, ErrorInfo> {
        let mut request = Request::empty();
        let mut mp = MultipartyThresholdRequest::empty();
        mp.multiparty_issue_unique_index = Some(MultipartyIssueUniqueIndex{ room_id: self.room_id.clone()});
        request.multiparty_threshold_request = Some(mp);
        let response = self.message(request).await?;
        let u = response.multiparty_threshold_response.safe_get()?
            .multiparty_issue_unique_index_response.safe_get()?
            .unique_index;
        Ok(u as u16)
    }

    pub async fn broadcast(&self, message: &str) -> Result<(), ErrorInfo> {

        let mut request = Request::empty();
        let mut mp = MultipartyThresholdRequest::empty();
        mp.multiparty_broadcast = Some(MultipartyBroadcast{ room_id: self.room_id.clone(), message: message.to_string() });
        request.multiparty_threshold_request = Some(mp);
        let response = self.message(request).await?;
        response.multiparty_threshold_response.safe_get()?
            .multiparty_issue_unique_index_response.safe_get()?
            .unique_index;
        response.as_error_info()?;
        Ok(())
    }

    pub fn process_subscription(receiver: &flume::Receiver<MultipartySubscribeEvent>)
        -> Result<impl Stream<Item = Result<String, ErrorInfo>> + '_, ErrorInfo> {
        let stream = receiver.stream()
            .map(|e| // move
                {
                    Ok(e.message)
                    // if e.room_id == rid {
                    //     Some(Ok(e.message))
                    // } else {
                    //     None
                    // }
                });
        Ok(stream)
    }

    pub async fn subscribe(self) -> Result<(), ErrorInfo> {

        let mut sub_internal = MultipartyRequestResponse::empty();
        sub_internal.internal_subscribe = Some(MultipartyRoomInternalSubscribe{
            room_id: self.room_id.clone(),
            sender: self.sub_channel.sender.clone(),
        });
        self.relay.multiparty.sender.send_err(sub_internal)?;

        let mut request = Request::empty();
        let mut mp = MultipartyThresholdRequest::empty();
        mp.multiparty_subscribe = Some(
            MultipartySubscribe{
                room_id: self.room_id.clone(),
                last_event_id: None,
                shutdown: false,
            });
        request.multiparty_threshold_request = Some(mp);
        let response = self.message(request).await?;
        response.as_error_info()?;
        let rid = self.room_id.clone();
        // TODO Alternate mechanism here is try stream!{ rec.next() } pattern
        Ok(())
    }
}




//
// #[derive(Deserialize, Debug)]
// struct IssuedUniqueIdx {
//     unique_idx: u16,
// }
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
// #[tokio::main]
// #[allow(dead_code)]
// async fn main() -> Result<()> {
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
