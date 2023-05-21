use log::info;
use metrics::increment_counter;
use redgold_schema::structs::ErrorInfo;
use redgold_schema::util;
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::core::relay::Relay;
use redgold_schema::EasyJson;

#[derive(Clone)]
pub struct ObservationHandler {
    pub relay: Relay
}

impl ObservationHandler {

    pub async fn run(&self) -> Result<(), ErrorInfo> {
        loop {
            increment_counter!("redgold.observation.received");
            let o = self.relay.observation.receiver.recv_async_err().await?;
            info!("Received peer observation {}", serde_json::to_string(&o.clone()).unwrap());
            // TODO: Verify merkle root
            // TODO: Verify time and/or avoid updating time if row already present.
            let option = o.clone().struct_metadata.map(|s| s.time).unwrap_or(util::current_time_millis());
            let res = self.relay.ds.observation.insert_observation_and_edges(&o, option).await;
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Insert observation error from received observation: {}", e.json_or())
                }
            }
        }
    }
}