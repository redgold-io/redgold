// use std::sync::{
//     atomic::{AtomicU16, Ordering},
//     Arc,
// };
// use log::info;
// use tokio::sync::{Notify, RwLock};
// use warp::Filter;
// use redgold_schema::structs::{ErrorInfo, MultipartyIssueUniqueIndexResponse, MultipartySubscribeEvent, MultipartyThresholdRequest, MultipartyThresholdResponse, NodeMetadata, PublicKey};
// use crate::core::relay::{MultipartyRequestResponse, Relay};
// use std::collections::hash_map::{Entry, HashMap};
// use std::time::Duration;
// use metrics::increment_counter;
// use tokio::runtime::Runtime;
// use tokio::select;
// use tokio::task::JoinHandle;
// use tokio::time::error::Elapsed;
// use redgold_schema::{error_info, SafeOption, structs};
// use crate::core::internal_message::{Channel, FutLoopPoll, RecvAsyncErrorInfo, SendErrorInfo};
//
// use futures::stream::{FuturesUnordered, StreamExt};
// use libp2p::autonat::InboundProbeEvent::Request;
//
//
// pub struct Subscription {
//     room: Arc<Room>,
//     next_event: u16,
// }
//
// impl Subscription {
//     pub async fn next(&mut self) -> (u16, String) {
//         loop {
//             let history = self.room.messages.read().await;
//             if let Some(msg) = history.get(usize::from(self.next_event)) {
//                 let event_id = self.next_event;
//                 self.next_event = event_id + 1;
//                 return (event_id, msg.clone());
//             }
//             let notification = self.room.message_appeared.notified();
//             drop(history);
//             notification.await;
//         }
//     }
// }
//
// impl Drop for Subscription {
//     fn drop(&mut self) {
//         self.room.subscribers.fetch_sub(1, Ordering::SeqCst);
//     }
// }
//
// /// Represents a header Last-Event-ID
// struct LastEventId(Option<u16>);
//
//
// pub struct Db {
//     rooms: RwLock<HashMap<String, Arc<Room>>>,
// }
//
// pub struct Room {
//     messages: RwLock<Vec<String>>,
//     message_appeared: Notify,
//     subscribers: AtomicU16,
//     next_idx: AtomicU16,
// }
//
// impl Db {
//     pub fn empty() -> Self {
//         Self {
//             rooms: RwLock::new(HashMap::new()),
//         }
//     }
//
//     pub async fn get_room_or_create_empty(&self, room_id: &str) -> Arc<Room> {
//         let rooms = self.rooms.read().await;
//         if let Some(room) = rooms.get(room_id) {
//             // If no one is watching this room - we need to clean it up first
//             if !room.is_abandoned() {
//                 return room.clone();
//             }
//         }
//         drop(rooms);
//
//         let mut rooms = self.rooms.write().await;
//         match rooms.entry(room_id.to_owned()) {
//             Entry::Occupied(entry) if !entry.get().is_abandoned() => entry.get().clone(),
//             Entry::Occupied(entry) => {
//                 let room = Arc::new(Room::empty());
//                 *entry.into_mut() = room.clone();
//                 room
//             }
//             Entry::Vacant(entry) => entry.insert(Arc::new(Room::empty())).clone(),
//         }
//     }
// }
//
// impl Room {
//     pub fn empty() -> Self {
//         Self {
//             messages: RwLock::new(vec![]),
//             message_appeared: Notify::new(),
//             subscribers: AtomicU16::new(0),
//             next_idx: AtomicU16::new(1),
//         }
//     }
//
//     pub async fn publish(self: &Arc<Self>, message: String) {
//         let mut messages = self.messages.write().await;
//         messages.push(message);
//         self.message_appeared.notify_waiters();
//     }
//
//     pub fn subscribe(self: Arc<Self>, last_seen_msg: Option<u16>) -> Subscription {
//         self.subscribers.fetch_add(1, Ordering::SeqCst);
//         Subscription {
//             room: self,
//             next_event: last_seen_msg.map(|i| i + 1).unwrap_or(0),
//         }
//     }
//
//     pub fn is_abandoned(&self) -> bool {
//         self.subscribers.load(Ordering::SeqCst) == 0
//     }
//
//     pub fn issue_unique_idx(&self) -> u16 {
//         self.next_idx.fetch_add(1, Ordering::Relaxed)
//     }
// }
//
//
// pub struct MpartyServer {
//     pub relay: Relay,
//     pub db: Db
// }
//
// /*
//
// Is this even necessary? Can't we make this a public request type?
//  */
// //
// // impl MpartyServer {
// //
// //     pub async fn run_server(relay: Relay) -> Result<(), ErrorInfo>{
// //         let relay2 = relay.clone();
// //
// //         let hello = warp::get()
// //             .and(warp::path("hello")).and_then(|| async move {
// //             let res: Result<&str, warp::reject::Rejection> = Ok("hello");
// //             res
// //         });
// //
// //         let port = relay.node_config.mparty_port();
// //         info!("Running Mpartyserver API on port: {:?}", port.clone());
// //         Ok(warp::serve(hello)
// //             .run(([0, 0, 0, 0], port))
// //             .await)
// //     }
// //
// //     // pub fn start(relay: Relay, arc: Arc<Runtime>) -> JoinHandle<Result<(), ErrorInfo>> {
// //     //     let mut o = Self {
// //     //         relay
// //     //     };
// //     //     arc.spawn(async move {
// //     //         o.run().await
// //     //     })
// //     // }
// // }
//
//
// pub struct MultipartyHandler {
//     pub relay: Relay,
//     pub db: Db,
//     // pub rt: Arc<Runtime>,
//     pub subscribers: HashMap<(Vec<u8>, String), JoinHandle<Result<(), ErrorInfo>>>,
//     // TODO: Unify potentially with FutLoopPoll
//     pub futs: FuturesUnordered<JoinHandle<Result<(), ErrorInfo>>>,
//     pub internal_subscribers: HashMap<String, flume::Sender<MultipartySubscribeEvent>>,
// }
//
//
//
// impl MultipartyHandler {
//
//     pub fn new(relay: Relay
//                // , rt: Arc<Runtime>
//     ) -> Self {
//         Self {
//             relay,
//             db: Db::empty(),
//             // rt,
//             subscribers: HashMap::new(),
//             futs: FuturesUnordered::new(),
//             internal_subscribers: Default::default(),
//         }
//     }
//
//     // TODO: Finish here
//     pub async fn handle_subscription(mut subscription: Subscription, node_key: PublicKey, relay: Relay, room_id: String) -> Result<(), ErrorInfo> {
//         let result = tokio::time::timeout(
//             Duration::from_secs(300),
//             async move {
//                 //let nmd2 = node_key.clone();
//                 loop {
//                     let (id, msg) = subscription.next().await;
//                     let ev = MultipartySubscribeEvent{
//                         room_id: room_id.clone(),
//                         id: id.to_string(),
//                         message: msg,
//                     };
//                     let mut req = structs::Request::empty();
//                     let mut mpr = structs::MultipartyThresholdRequest::empty();
//                     mpr.multiparty_subscribe_events = vec![ev];
//                     relay.send_message(req, node_key.clone()).await;
//                 }
//             }).await;
//         result.map_err(|e| error_info(format!("elapsed {}", e.to_string())))
//     }
//
//     pub async fn run(&mut self) -> Result<(), ErrorInfo> {
//         loop {
//             select! {
//                 e = self.futs.next() => {
//                     FutLoopPoll::map_fut(e)?
//                 }
//                 oo = self.relay.multiparty.receiver.recv_async_err() => {
//                 increment_counter!("redgold.multiparty.received");
//                 let o: MultipartyRequestResponse = oo?;
//                 if let Some(r) = o.request {
//                     if let Some(rr) = r.multiparty_broadcast {
//                         let room = self.db.get_room_or_create_empty(&*rr.room_id).await;
//                         room.publish(rr.message).await;
//                     }
//                     if let Some(rr) = r.multiparty_issue_unique_index {
//                         let room = self.db.get_room_or_create_empty(&*rr.room_id).await;
//                         let idx = room.issue_unique_idx();
//                         if let Some(s) = o.sender {
//                             // let response = MultipartyThresholdResponse {
//                             //     multiparty_issue_unique_index_response: Some(MultipartyIssueUniqueIndexResponse{
//                             //         unique_index: idx as i64,
//                             //     }),
//                             // };
//                             // s.send_err(response)?;
//                         }
//                     }
//                     if let Some(rr) = r.multiparty_subscribe {
//                         if let Some(n) = o.origin {
//                             if let Some(pk)  = n.public_key {
//                             if rr.shutdown {
//                                 // let nmd = n.clone();
//                                 // let option = nmd.public_key.safe_get()?.clone();
//                                 // let vec = option.bytes.safe_get()?.value.clone();
//                                 // if let Some(jh) = self.subscribers.get(&(vec, rr.room_id)) {
//                                 //     jh.abort();
//                                 //     info!("Aborted subscribe")
//                                 // }
//                             } else {
//                             let room = self.db.get_room_or_create_empty(&*rr.room_id).await;
//                             let mut subscription = room.subscribe(
//                                 rr.last_event_id.map(|i| i as u16)
//                             );
//                             // Need a timeout on the subscription length, send messages over the REST API
//                             // back to the peer id of interest that is subscribed
//                             // TODO: Change to FutLoopPoll.
//                             let jh = self.rt.spawn(Self::handle_subscription(
//                                             subscription, pk, self.relay.clone(), rr.room_id.clone()
//                                         ));
//
//                             }
//                         }
//                     }
//
//                     }
//                     for event in r.multiparty_subscribe_events {
//                             if let Some(sub) = self.internal_subscribers.get(&event.room_id) {
//                                 sub.send(event);
//                             }
//                     }
//                 }
//
//                 }
//
//             }
//
//             // if let Some(resp) = o.response {
//             //     resp.multiparty_issue_unique_index_response
//             //     ()
//             // }
//         }
//     }
// }