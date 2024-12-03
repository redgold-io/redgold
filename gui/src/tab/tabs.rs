use strum_macros::EnumIter;
use serde::{Deserialize, Serialize};

#[derive(Debug, EnumIter, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum Tab {
    Home,
    Transact,
    Portfolio,
    Keys,
    Address,
    Contacts,
    Servers,
    Settings,
    Ratings,
    Identity,
    OTP,
    Airgap
}