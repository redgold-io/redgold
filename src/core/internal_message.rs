use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;
use crate::schema::structs::{ErrorCode, ErrorInfo, PublicResponse, Request, Response, Transaction};
use crate::schema::error_message;
use bdk::bitcoin::secp256k1::PublicKey;
use tokio::task::JoinError;
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
    pub origin: MessageOrigin
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
use async_trait::async_trait;
use flume::TryRecvError;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::select;
use tokio::task::JoinHandle;
use redgold_schema::{error_info, ErrorInfoContext, structs};
use redgold_schema::structs::{DynamicNodeMetadata, NodeMetadata};
use crate::api::rosetta::models::Peer;
use redgold_schema::conf::node_config::NodeConfig;

#[async_trait]
pub trait RecvAsyncErrorInfo<T> {
    async fn recv_async_err(&self) -> Result<T, ErrorInfo>;
    async fn recv_async_err_timeout(&self, timeout: Duration) -> Result<T, ErrorInfo>;
}

#[async_trait]
impl<T> RecvAsyncErrorInfo<T> for flume::Receiver<T>
where
    T: Send,
{
    async fn recv_async_err(&self) -> Result<T, ErrorInfo> {
        self.recv_async()
            .await
            .map_err(|e| error_message(ErrorCode::InternalChannelReceiveError, e.to_string()))
    }
    async fn recv_async_err_timeout(&self, duration: Duration) -> Result<T, ErrorInfo> {
        tokio::time::timeout(duration, self.recv_async_err())
            .await
            .error_info("Timeout recv async error")?
    }

}

#[async_trait]
pub trait SendErrorInfo<T> {
    fn send_rg_err(&self, t: T) -> Result<(), ErrorInfo>;
}

#[async_trait]
impl<T> SendErrorInfo<T> for flume::Sender<T>
where
    T: Send,
{
    fn send_rg_err(&self, t: T) -> Result<(), ErrorInfo> {
        self.send(t)
            .map_err(|e| error_message(ErrorCode::InternalChannelReceiveError, e.to_string()))
    }
}

#[derive(Clone)]
pub struct Channel<T> {
    pub sender: flume::Sender<T>,
    pub receiver: flume::Receiver<T>,
}

impl<T> Channel<T> {
    pub async fn send(&self, t: T) -> Result<(), ErrorInfo> {
        self.sender
            .send(t)
            .map_err(|e| error_message(ErrorCode::InternalChannelSendError, e.to_string()))
    }
    pub fn new() -> Channel<T> {
        new_channel()
    }

    pub fn recv_while(&self) -> Result<Vec<T>, ErrorInfo> {
        let mut results = vec![];
        while {
            let err = self.receiver.try_recv();
            let mut continue_loop = true;
            match err {
                Ok(o) => {
                    results.push(o);
                }
                Err(e) => {
                    match e {
                        TryRecvError::Empty => {
                            continue_loop = false;
                        }
                        TryRecvError::Disconnected => {
                            return Err(error_info("request processor channel closed unexpectedly"));
                        }
                    }
                }
            }
            continue_loop
        } {}
        Ok(results)
    }
}

pub fn new_channel<T>() -> Channel<T> {
    let (s, r) = flume::unbounded::<T>();
    return Channel {
        sender: s,
        receiver: r,
    };
}


pub fn new_bounded_channel<T>(cap: usize) -> Channel<T> {
    let (s, r) = flume::bounded::<T>(cap);
    return Channel {
        sender: s,
        receiver: r,
    };
}


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