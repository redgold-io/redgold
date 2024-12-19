use std::net::SocketAddr;
use std::time::Duration;
use async_trait::async_trait;
use crate::schema::structs::{ErrorInfo, Request, Response, Transaction};
use bdk::bitcoin::secp256k1::PublicKey;
use tokio::task::JoinError;
use redgold_common::flume_send_help::RecvAsyncErrorInfo;
// #[derive(Clone)]
// pub struct InternalChannel<T> {
//     pub sender: flume::Sender<T>,
//     pub receiver: flume::Receiver<T>,
// }

// pub fn new_internal_channel<T>() -> Channel<T> {
//     let (s, r) = crossbeam_channel::unbounded::<T>();
//     return Channel {
//         sender: s,
//         receiver: r,
//     };
// }

#[derive(Clone)]
pub enum MessageOrigin {
    Udp,
    Rest
}

/// Bidirectional message type
#[derive(Clone)]
pub struct PeerMessage {
    pub request: Request,
    // TODO: Change this to RgResult<Response> to disambiguate?
    pub response: Option<flume::Sender<Response>>,
    pub public_key: Option<structs::PublicKey>,
    pub socket_addr: Option<SocketAddr>,
    pub destinations: Vec<PublicKey>,
    pub node_metadata: Option<NodeMetadata>,
    pub dynamic_node_metadata: Option<DynamicNodeMetadata>,
    pub send_timeout: Duration,
    pub origin: MessageOrigin,
    pub requested_transport: Option<TransportBackend>
}

impl Default for PeerMessage {
    fn default() -> Self {
        Self::empty()
    }
}

impl PeerMessage {
    pub fn empty() -> Self {
        Self{
            request: Request::empty(),
            response: None,
            public_key: None,
            socket_addr: None,
            destinations: vec![],
            node_metadata: None,
            dynamic_node_metadata: None,
            send_timeout: Duration::from_secs(150),
            origin: MessageOrigin::Rest,
            requested_transport: None,
        }
    }

    pub fn from_metadata(request: Request, metadata: NodeMetadata) -> Self {
        let mut mt = Self::empty();
        mt.request = request;
        mt.node_metadata = Some(metadata);
        mt
    }

    pub fn from_pk(request: &Request, pk: &structs::PublicKey) -> Self {
        let mut mt = Self::empty();
        mt.request = request.clone();
        mt.public_key = Some(pk.clone());
        mt
    }

    // pub async fn send(nc: NodeConfig) -> Result<Response, Error> {
    //     let req = nc.request();
    //     let c = new_channel::<Response>();
    //     let mut pm = Self::empty();
    //     pm.request = req;
    //     pm.response = Some(c.sender);
    //
    // }
}

// Some other field was needed here but I can't remember what it was
#[derive(Clone, Debug)]
pub struct TransactionMessage {
    pub transaction: Transaction,
    pub response_channel: Option<flume::Sender<Response>>,
    pub origin: Option<structs::PublicKey>,
    pub origin_ip: Option<String>
}
use redgold_schema::{structs, ErrorInfoContext};
use redgold_schema::structs::{DynamicNodeMetadata, NodeMetadata, TransportBackend};
pub fn map_fut(r: Option<Result<Result<(), ErrorInfo>, JoinError>>) -> Result<(), ErrorInfo> {
    match r {
        None => {
            Ok(())
        }
        Some(resres) => {
            resres.map_err(|je| ErrorInfo::error_info(
                format!("Panic in loop runner thread {}", je.to_string())
            ))??;
            Ok(())
        }
    }
}

#[async_trait]
pub trait RecvAsyncErrorInfoTimeout<T> {
    async fn recv_async_err_timeout(&self, timeout: Duration) -> Result<T, ErrorInfo>;
}

#[async_trait]
impl<T> RecvAsyncErrorInfoTimeout<T> for flume::Receiver<T>
where
    T: Send,
{
    async fn recv_async_err_timeout(&self, duration: Duration) -> Result<T, ErrorInfo> {
        tokio::time::timeout(duration, self.recv_async_err())
            .await
            .error_info("Timeout recv async error")?
    }
}