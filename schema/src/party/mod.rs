pub mod party_info;
pub mod all_parties;

use crate::structs::RoomId;

impl RoomId {
    pub fn from(uuid: String) -> Self {
        Self {
            uuid: Some(uuid),
        }
    }
}