use strum_macros::EnumIter;
use serde::{Deserialize, Serialize};

#[derive(Debug, EnumIter, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum Tab {
    Home,
    Keys,
    Transact,
    Portfolio,
    Identity,
    Contacts,
    Address,
    Servers,
    Ratings,
    Settings,
    OTP,
}