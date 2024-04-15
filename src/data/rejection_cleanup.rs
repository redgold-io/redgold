// use async_trait::async_trait;
// use redgold_schema::RgResult;
// use crate::core::relay::Relay;
// use crate::core::stream_handlers::IntervalFold;
//
// pub struct RejectionCleanup {
//     relay: Relay,
// }
//
// impl RejectionCleanup {
//     pub fn new(relay: &Relay) -> Self {
//         Self { relay: relay.clone() }
//     }
// }
//
// #[async_trait]
// impl IntervalFold for RejectionCleanup {
//     async fn interval_fold(&mut self) -> RgResult<()> {
//         self.relay.ds.transaction_store.query_rejected_transaction()
//         Ok(())
//     }
// }