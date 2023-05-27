use futures::{StreamExt, TryStreamExt};
use log::info;
use metrics::increment_counter;
use redgold_schema::structs::{ErrorInfo, Observation};
use redgold_schema::{util, WithMetadataHashable};
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::core::relay::Relay;
use redgold_schema::EasyJson;

#[derive(Clone)]
pub struct ObservationHandler {
    pub relay: Relay
}

impl ObservationHandler {

    async fn process_message(&self, o: &Observation) -> Result<(), ErrorInfo> {
        increment_counter!("redgold.observation.received");
        info!("Received peer observation {}", o.json_or());
        // TODO: Verify merkle root
        // TODO: Verify time and/or avoid updating time if row already present.
        let i = util::current_time_millis();
        let option = o.time()
            .unwrap_or(&i);
        self.relay.ds.observation.insert_observation_and_edges(&o, option.clone()).await?;
        Ok(())
    }

    async fn fold(&self, o: Observation) -> Result<&Self, ErrorInfo> {
        self.process_message(&o).await.map(|_| self)
    }

        // TODO: Pass in the dependencies directly.
    pub async fn run(&self) -> Result<(), ErrorInfo> {
        let receiver = self.relay.observation.receiver.clone();
        receiver.into_stream().map(Ok).try_fold(self, |s, o| {s.fold(o)
        }).await.map(|_| ())
    }
}