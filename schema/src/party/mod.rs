pub mod party_info;
pub mod all_parties;
pub mod external_data;
pub mod address_event;
pub mod party_events;
pub mod central_price;
pub mod price_volume;
pub mod portfolio;

use crate::structs::RoomId;

impl RoomId {
    pub fn from(uuid: String) -> Self {
        Self {
            uuid: Some(uuid),
        }
    }
}