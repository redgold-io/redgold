use uuid::Uuid;
use crate::{bytes_data, HashClear, util};
use crate::structs::UdpMessage;

impl HashClear for UdpMessage {
    fn hash_clear(&mut self) {}
}

impl UdpMessage {
    pub fn new(data: Vec<u8>, part: i64, parts: i64) -> Self {
        UdpMessage {
            bytes: bytes_data(data),
            part,
            parts,
            uuid: Uuid::new_v4().to_string(),
            timestamp: util::current_time_millis(),
        }
    }
}