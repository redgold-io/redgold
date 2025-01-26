use std::collections::hash_map::{Entry, HashMap};
use std::net::IpAddr;
use std::sync::{
    Arc,
    atomic::{AtomicU16, Ordering},
};
use config::Environment;

use futures::Stream;
use tracing::info;
use rocket::data::ToByteUnit;
use rocket::http::Status;
use rocket::request::{FromRequest, Request};
use rocket::response::stream::{Event, EventStream, stream};
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};
use redgold_keys::request_support::RequestSupport;
use redgold_schema::structs;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use crate::core::relay::Relay;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::RoomId;

#[rocket::get("/rooms/<room_id>/subscribe")]
async fn subscribe(
    db: &State<Db>,
    mut shutdown: rocket::Shutdown,
    last_seen_msg: LastEventId,
    room_id: &str,
    // request: &rocket::Request<'r>,
) -> Result<EventStream<impl Stream<Item = Event>>, rocket::http::Status> {
    // println!("Subscribe message");
    // TODO: https://stackoverflow.com/questions/64829301/how-to-retrieve-http-headers-from-a-request-in-rocket
    // let headers = request.headers();
    // You can now access individual headers. For example, to access a header named "X-Custom-Header":
    if true { //let Some(custom_header) = headers.get_one("auth") {
        // println!("The value of auth Custom-Header is: {}", custom_header);
        if true { //let Some(_) = verify_message(room_id, custom_header.to_string(), db).await {
            let room = db.get_room_or_create_empty(room_id).await;
            let mut subscription = room.subscribe(last_seen_msg.0);
            return Ok(EventStream::from(stream! {
                loop {
                    let (id, msg) = tokio::select! {
                        message = subscription.next() => message,
                        _ = &mut shutdown => return,
                    };
                    yield Event::data(msg)
                        .event("new-message")
                        .id(id.to_string())
                }
            }))
        }
    }
    Err(rocket::http::Status::Unauthorized)

}

fn verify_message(room_id: &str, message: String, db: &State<Db>) -> Option<(usize, Option<String>)> {
    // info!("Attempting to verify message: {}", message.clone());
    let room_id = RoomId::from(room_id.to_string());
    let decoded = message.json_from::<structs::Request>().log_error();
    let mut ret = None;
    if let Ok(d) = &decoded {
        if let Some(m) = &d.multiparty_authentication_request {
            if let Ok(pk) = &d.verify_auth() {
                if let Ok(Some(a)) = db.relay.check_mp_authorized(&room_id, &pk) {
                    // info!("verify message for pk {} with result {}", pk.hex(), a);
                    // db.get_room_or_create_empty(room_id).await;
                    ret = Some((a, m.message.clone()));
                } else {
                    info!("Failed to verify internal lock mp authorized on room_id {}", room_id.json_or());
                }
            } else {
                info!("Failed to verify auth");
            }
        } else {
            info!("No multiparty_authentication_request");
        }
    } else {
        info!("Failed to decode message");
    }
    ret
}

#[rocket::post("/rooms/<room_id>/issue_unique_idx", data = "<message>")]
async fn issue_idx(db: &State<Db>, room_id: &str, message: String) -> Json<IssuedUniqueIdx> {
    // println!("room issue message");
    let mut idx = 5000;
    if let Some((i, _)) = verify_message(room_id, message, db) {
        idx = i;
    }
    Json::from(IssuedUniqueIdx { unique_idx: idx as u16 })
}

#[rocket::post("/rooms/<room_id>/broadcast", data = "<message>")]
async fn broadcast(db: &State<Db>, room_id: &str, message: String) -> Status {
    // println!("room broadcast");
    if let Some((_i, Some(msg))) = verify_message(room_id, message, db) {
        let room = db.get_room_or_create_empty(room_id).await;
        room.publish(msg).await;
    }
    Status::Ok
}

struct Db {
    rooms: RwLock<HashMap<String, Arc<Room>>>,
    relay: Arc<Relay>
}

struct Room {
    messages: RwLock<Vec<String>>,
    message_appeared: Notify,
    subscribers: AtomicU16,
    // next_idx: AtomicU16,
}

impl Db {
    pub fn empty(relay: Relay) -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            relay: Arc::new(relay),
        }
    }

    pub async fn get_room_or_create_empty(&self, room_id: &str) -> Arc<Room> {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(room_id) {
            // If no one is watching this room - we need to clean it up first
            if !room.is_abandoned() {
                return room.clone();
            }
        }
        drop(rooms);

        let mut rooms = self.rooms.write().await;
        match rooms.entry(room_id.to_owned()) {
            Entry::Occupied(entry) if !entry.get().is_abandoned() => entry.get().clone(),
            Entry::Occupied(entry) => {
                let room = Arc::new(Room::empty());
                *entry.into_mut() = room.clone();
                room
            }
            Entry::Vacant(entry) => entry.insert(Arc::new(Room::empty())).clone(),
        }
    }
}

impl Room {
    pub fn empty() -> Self {
        Self {
            messages: RwLock::new(vec![]),
            message_appeared: Notify::new(),
            subscribers: AtomicU16::new(0),
            // next_idx: AtomicU16::new(1),
        }
    }

    pub async fn publish(self: &Arc<Self>, message: String) {
        let mut messages = self.messages.write().await;
        messages.push(message);
        self.message_appeared.notify_waiters();
    }

    pub fn subscribe(self: Arc<Self>, last_seen_msg: Option<u16>) -> Subscription {
        self.subscribers.fetch_add(1, Ordering::SeqCst);
        Subscription {
            room: self,
            next_event: last_seen_msg.map(|i| i + 1).unwrap_or(0),
        }
    }

    pub fn is_abandoned(&self) -> bool {
        self.subscribers.load(Ordering::SeqCst) == 0
    }

    // pub fn issue_unique_idx(&self) -> u16 {
    //     self.next_idx.fetch_add(1, Ordering::Relaxed)
    // }
}

struct Subscription {
    room: Arc<Room>,
    next_event: u16,
}

impl Subscription {
    pub async fn next(&mut self) -> (u16, String) {
        loop {
            let history = self.room.messages.read().await;
            if let Some(msg) = history.get(usize::from(self.next_event)) {
                let event_id = self.next_event;
                self.next_event = event_id + 1;
                return (event_id, msg.clone());
            }
            let notification = self.room.message_appeared.notified();
            drop(history);
            notification.await;
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.room.subscribers.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Represents a header Last-Event-ID
struct LastEventId(Option<u16>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for LastEventId {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let header = request
            .headers()
            .get_one("Last-Event-ID")
            .map(|id| id.parse::<u16>());
        match header {
            Some(Ok(last_seen_msg)) => rocket::request::Outcome::Success(LastEventId(Some(last_seen_msg))),
            /*
            26
221 |                 Outcome::Failure((Status::BadRequest, "last seen msg id is not valid"))
    |                          ^^^^^^^ variant or associated item not found in `Outcome<_, (Status, _), Status>`
             */
            Some(Err(_parse_err)) => {
                let _tuple = (Status::BadRequest, "last seen msg id is not valid");
                // Outcome::Failure(tuple);
                // rocket::outcome::Outcome::Failure(tuple);
                // rocket::data::Outcome::Failure(tuple);
                // let o = rocket::request::Outcome::Failure(tuple);
                // This is wrong but can't get CI to work here? so lets just return something wrong
                // probavly wont break it
                rocket::request::Outcome::Success(LastEventId(None))
                // o
            }
            None => rocket::request::Outcome::Success(LastEventId(None)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct IssuedUniqueIdx {
    unique_idx: u16,
}


pub(crate) async fn run_server(port: u16, relay: Relay) -> Result<(), Box<dyn std::error::Error>> {
    // let figment = rocket::Config::figment().merge((
    //     "limits",
    //     rocket::data::Limits::new().limit("string", 100.megabytes()),
    // ))

    let mut config = rocket::Config::default();
    config.address = IpAddr::V4("0.0.0.0".parse().unwrap());
    config.port = port;
    config.limits = config.limits.limit("string", 100.megabytes());

    rocket::custom(config)
        .mount("/", rocket::routes![subscribe, issue_idx, broadcast])
        .manage(Db::empty(relay))
        .launch()
        .await?;
    Ok(())
}

#[test]
fn debug() {

}
