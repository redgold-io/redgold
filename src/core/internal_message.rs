use std::future::Future;
use std::net::SocketAddr;
use crate::schema::structs::{Error, ErrorInfo, PublicResponse, Request, Response, Transaction};
use crate::schema::error_message;
use bitcoin::secp256k1::PublicKey;
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
pub struct PeerMessage {
    pub request: Request,
    pub response: Option<flume::Sender<Response>>,
    pub public_key: Option<PublicKey>,
    pub socket_addr: Option<SocketAddr>
}

impl PeerMessage {
    pub fn empty() -> Self {
        Self{
            request: Request::empty(),
            response: None,
            public_key: None,
            socket_addr: None
        }
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

#[derive(Clone)]
pub struct TransactionMessage {
    pub transaction: Transaction,
    pub response_channel: Option<flume::Sender<PublicResponse>>,
}
use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::select;
use tokio::task::JoinHandle;
use crate::api::rosetta::models::Peer;
use crate::node_config::NodeConfig;

#[async_trait]
pub trait RecvAsyncErrorInfo<T> {
    async fn recv_async_err(&self) -> Result<T, ErrorInfo>;
}

#[async_trait]
impl<T> RecvAsyncErrorInfo<T> for flume::Receiver<T>
where
    T: Send,
{
    async fn recv_async_err(&self) -> Result<T, ErrorInfo> {
        self.recv_async()
            .await
            .map_err(|e| error_message(Error::InternalChannelReceiveError, e.to_string()))
    }
}

#[async_trait]
pub trait SendErrorInfo<T> {
    fn send_err(&self, t: T) -> Result<(), ErrorInfo>;
}

#[async_trait]
impl<T> SendErrorInfo<T> for flume::Sender<T>
where
    T: Send,
{
    fn send_err(&self, t: T) -> Result<(), ErrorInfo> {
        self.send(t)
            .map_err(|e| error_message(Error::InternalChannelReceiveError, e.to_string()))
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
            .map_err(|e| error_message(Error::InternalChannelSendError, e.to_string()))
    }
    pub fn new() -> Channel<T> {
        new_channel()
    }
}

pub fn new_channel<T>() -> Channel<T> {
    let (s, r) = flume::unbounded::<T>();
    return Channel {
        sender: s,
        receiver: r,
    };
}


pub struct FutLoopPoll {
    pub futures: FuturesUnordered<JoinHandle<Result<(), ErrorInfo>>>
}

impl FutLoopPoll {

    pub fn new() -> FutLoopPoll {
        FutLoopPoll {
            futures: FuturesUnordered::new()
        }
    }

    pub async fn run<T, F>(
        &mut self, receiver: flume::Receiver<T>, func: F
    )-> Result<(), ErrorInfo>
        where T: Sized + Send,
    F: FnOnce(T) -> JoinHandle<Result<(), ErrorInfo>> + Copy
    {

        let mut futures = &mut self.futures;

        loop {
            let loop_sel_res = select! {
                msg = receiver.recv_async_err() => {
                    let msg_actual: T = msg?;
                    futures.push(func(msg_actual));
                    Ok(())
                }
                res = futures.next() => {
                    Self::map_fut(res)?;
                    Ok(())
                }
            };
            loop_sel_res?;
        }
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

    pub async fn run_fut<T, F, Fut, FutBound>(
        &mut self, fut: Fut, func: F
    )-> Result<(), ErrorInfo>
        where
        FutBound: Future<Output=T> + Sized,
    Fut: (FnOnce() -> FutBound) + Copy,
    F: FnOnce(T) -> JoinHandle<Result<(), ErrorInfo>> + Copy
    {

        let mut futures = &mut self.futures;

        loop {
            let loop_sel_res = select! {
                msg = fut() => {
                    futures.push(func(msg));
                    Ok(())
                }
                res = futures.next() => {
                    Self::map_fut(res)?;
                    Ok(())
                }
            };
            loop_sel_res?;
        }
    }


}