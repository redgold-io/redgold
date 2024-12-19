use std::time::Duration;
use async_trait::async_trait;
use itertools::Itertools;
use tokio::task::JoinHandle;
use redgold_common::flume_send_help::Channel;
use redgold_schema::RgResult;
use redgold_schema::structs::{KeepAliveRequest, PublicKey, Request};
use crate::core::internal_message::PeerMessage;
use crate::core::relay::Relay;
use crate::core::stream_handlers::{run_interval_fold, IntervalFold};
pub struct UdpKeepAlive {
    peer_outgoing: Channel<PeerMessage>,
    keep_alive_peers: Vec<PublicKey>,
    relay: Relay
}


impl UdpKeepAlive {
    pub fn new(
        peer_outgoing: &Channel<PeerMessage>,
        duration: Duration,
        keep_alive_peers: Vec<PublicKey>,
        relay: &Relay
    ) -> JoinHandle<RgResult<()>> {
        let s = Self {
            peer_outgoing: peer_outgoing.clone(),
            keep_alive_peers,
            relay: relay.clone(),
        };
        run_interval_fold(s, duration, false)
    }
}

#[async_trait]
impl IntervalFold for UdpKeepAlive {

    async fn interval_fold(&mut self) -> RgResult<()> {
        let mut kap = self.keep_alive_peers.clone();
        if kap.is_empty() {
            let seeds = self.relay.node_config.seeds_now();
            kap = seeds.into_iter().flat_map(|s| s.public_key).collect_vec()
        }
        for pk in &kap {
            let mut req = Request::default();
            req.keep_alive_request = Some(KeepAliveRequest::default());
            if let Some(r) = self.relay.send_udp(pk, &req, None).await.ok() {
                if let Some(r) = r.keep_alive_response {
                    let working_udp_port = r.port;
                    if let Ok(_) = self.relay.update_dynamic_node_metadata_active_udp_port(working_udp_port).await {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}