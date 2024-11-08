use crate::airgap::{AirgapMessage, SignInternal};
use crate::structs::Transaction;

impl AirgapMessage {
    pub fn sign(path: String, tx: Transaction) -> Self {
        let mut msg = AirgapMessage::default();
        let mut internal = SignInternal::default();
        internal.path = path;
        internal.txs.push(tx);
        msg.sign_internal = Some(internal);
        msg
    }
}