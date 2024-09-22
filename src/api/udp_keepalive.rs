use std::time::Duration;
use async_trait::async_trait;
use tokio::task::JoinHandle;
use redgold_schema::RgResult;
use redgold_schema::structs::{KeepAliveRequest, PublicKey, Request};
use crate::core::internal_message::{Channel, PeerMessage};
use crate::core::relay::Relay;
use crate::core::stream_handlers::{run_interval_fold, IntervalFold};
use crate::core::transport::peer_event_handler::PeerOutgoingEventHandler;

struct UdpKeepAlive {
    peer_outgoing: Channel<PeerMessage>,
    keep_alive_peers: Vec<PublicKey>,
    relay: Relay
}


impl UdpKeepAlive {
    pub async fn new(
        peer_outgoing: &Channel<PeerMessage>,
        duration: Option<Duration>,
        keep_alive_peers: Vec<PublicKey>,
        relay: &Relay
    ) -> JoinHandle<RgResult<()>> {
        let s = Self {
            peer_outgoing: peer_outgoing.clone(),
            keep_alive_peers,
            relay: relay.clone(),
        };
        run_interval_fold(s, duration.unwrap_or(Duration::from_secs(60)), false).await
    }
}

#[async_trait]
impl IntervalFold for UdpKeepAlive {

    async fn interval_fold(&mut self) -> RgResult<()> {
        let seeds = self.relay.node_config.seeds_now();
        for seed in seeds {
            if let Some(pk) = &seed.public_key {
                let mut req = Request::default();
                req.keep_alive_request = KeepAliveRequest::default();
                self.relay.send_message_await_response()
            }
        }
        Ok(())
    }
}