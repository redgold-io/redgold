use crate::structs::UdpMessage;
use crate::HashClear;

impl HashClear for UdpMessage {
    fn hash_clear(&mut self) {}
}
