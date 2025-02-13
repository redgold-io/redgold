pub mod party_info;
pub mod external_data;
pub mod address_event;
pub mod party_events;
pub mod central_price;
pub mod price_volume;
pub mod portfolio;
pub mod party_internal_data;
pub mod search_events;

use crate::structs::RoomId;

impl RoomId {
    pub fn from(uuid: String) -> Self {
        Self {
            uuid: Some(uuid),
        }
    }
}