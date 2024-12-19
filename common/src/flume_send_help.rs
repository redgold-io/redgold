use async_trait::async_trait;
use redgold_schema::{error_info, error_message, ErrorInfoContext, RgResult};
use redgold_schema::structs::{ErrorCode, ErrorInfo};
use std::time::Duration;
use flume::TryRecvError;

#[async_trait]
pub trait RecvAsyncErrorInfo<T> {
    async fn recv_async_err(&self) -> Result<T, ErrorInfo>;
    fn recv_err(&self) -> RgResult<T>;
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

    fn recv_err(&self) -> RgResult<T> {
        self.recv()
            .map_err(|e| error_message(ErrorCode::InternalChannelReceiveError, e.to_string()))
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