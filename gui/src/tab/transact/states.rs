use strum_macros::{EnumIter, EnumString};
use std::time::Instant;

#[derive(Debug, EnumIter, EnumString, PartialEq, Clone)]
#[repr(i32)]
pub enum WalletTab {
    Hardware,
    Software,
}

#[derive(Clone)]
pub struct DeviceListStatus {
    pub device_output: Option<String>,
    pub last_polled: Instant,
}

#[derive(Clone, PartialEq, EnumIter, EnumString, Debug)]
pub enum SendReceiveTabs {
    Home,
    Send,
    Receive,
    Swap,
    Stake,
    Portfolio,
    Custom,
}

impl Default for SendReceiveTabs {
    fn default() -> Self {
        SendReceiveTabs::Home
    }
}

#[derive(Clone, PartialEq, EnumString)]
pub enum CustomTransactionType {
    Swap,
    Stake
}