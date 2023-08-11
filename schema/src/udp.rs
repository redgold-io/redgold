use crate::{bytes_data, HashClear, util};
use crate::structs::UdpMessage;

impl HashClear for UdpMessage {
    fn hash_clear(&mut self) {}
}
