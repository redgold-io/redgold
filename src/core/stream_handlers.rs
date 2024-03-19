use std::time::Duration;
use redgold_schema::{RgResult, structs};
use tokio_stream::wrappers::IntervalStream;
use tokio::task::JoinHandle;
use async_trait::async_trait;
use std::collections::HashSet;
use std::future::Future;
use flume::Receiver;
use futures::future::Either;
use futures::TryStreamExt;
use tokio_stream::StreamExt;
use redgold_schema::structs::{ErrorInfo, GetPeersInfoRequest};
use tracing::error;
use redgold_schema::errors::EnhanceErrorInfo;
use crate::observability::logging::Loggable;

#[async_trait]
pub trait IntervalFold  {
    async fn interval_fold(&mut self) -> RgResult<()>;
}

pub async fn run_interval_fold(interval_f: impl IntervalFold + Send + 'static, interval_duration: Duration, run_at_start: bool)
    -> JoinHandle<RgResult<()>> {
    tokio::spawn(run_interval_inner(interval_f, interval_duration, run_at_start))
}

pub async fn run_interval_inner(
    interval_f: impl IntervalFold, interval_duration: Duration, run_at_start: bool
) -> RgResult<()> {
    let mut cs = interval_f;
    if run_at_start {
        cs.interval_fold().await?;
    }
    let interval1 = tokio::time::interval(interval_duration);
    IntervalStream::new(interval1)
        .map(|x| Ok(x))
        .try_fold(cs, |mut c, _| async {
            c.interval_fold().await.map(|_| c)
        }).await.map(|_| ())
}


#[async_trait]
pub trait RecvForEachConcurrent<T> {
    async fn recv_for_each(&mut self, message: T) -> RgResult<()>;
}
//
// pub async fn recv_run_inner<T: 'static>(recv_impl: impl RecvForEachConcurrent<T> + Clone + 'static + Send, recv: Receiver<T>, limit: usize) -> RgResult<()> {
//
// }

pub async fn run_recv<T: 'static + Send>(recv_impl: impl RecvForEachConcurrent<T>
+ Clone + 'static + Send + Sync, recv: Receiver<T>, limit: usize) -> JoinHandle<RgResult<()>> {
    let fut = recv.into_stream().map(|x| Ok(x))
        .try_for_each_concurrent(limit, move |m| {
            let mut s = recv_impl.clone();
            async move {
                s.recv_for_each(m).await
            }
        });
    tokio::spawn(fut)
}




#[async_trait]
pub trait IntervalFoldOrReceive<T> where T: Send + 'static {
    async fn interval_fold_or_recv(&mut self, message: Either<T, ()>) -> RgResult<()>;
    // async fn interval(&mut self) -> RgResult<()>;
    // async fn process_message(&mut self, message: T) -> RgResult<()>;

}


//
// #[async_trait]
// impl<T> OperationThing for dyn IntervalFoldOrReceive<T> where T: Send + 'static {
//     async fn interval_fold_or_recv(&mut self, message: Either<T, ()>) -> RgResult<()> {
//
//     }
//
// }

pub async fn run_interval_fold_or_recv<T>(
    interval_f: impl IntervalFoldOrReceive<T> + Send + 'static,
    interval_duration: Duration, run_at_start: bool, recv: flume::Receiver<T>
) -> JoinHandle<RgResult<()>> where T: Send + 'static {
    tokio::spawn(run_interval_inner_or_recv::<T>(interval_f, interval_duration, run_at_start, recv))
}

pub async fn run_interval_inner_or_recv<T>(
    interval_f: impl IntervalFoldOrReceive<T>, interval_duration: Duration, run_at_start: bool,
    receiver: Receiver<T>
) -> RgResult<()> where T: Send + 'static {
    let mut cs = interval_f;
    if run_at_start {
        cs.interval_fold_or_recv(Either::Right(())).await?;
    }

    let recv = receiver.clone();
    let stream = recv
        .into_stream()
        .map(|x| Ok(Either::Left(x)));
    let interval = tokio::time::interval(interval_duration);
    let interval_stream = IntervalStream::new(interval).map(|_| Ok(Either::Right(())));

    stream.merge(interval_stream).try_fold(
        cs, |mut ob, o| async {
            ob.interval_fold_or_recv(o).await.map(|_| ob)
        }
    ).await.map(|_| ())
}
