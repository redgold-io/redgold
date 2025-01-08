use tokio::task::JoinHandle;
use flume::Sender;
use redgold_common::flume_send_help::{Channel, RecvAsyncErrorInfo};
use tracing::{error, info};

pub fn log_handler() -> (JoinHandle<()>, Option<Sender<String>>) {
    let c: Channel::<String> = Channel::new();
    let r = c.receiver.clone();
    let default_fun = tokio::spawn(async move {
        loop {
            let s = match r.recv_async_err().await {
                Ok(x) => {
                    x
                }
                Err(e) => {
                    error!("Error in deploy: {}", e.json_or());
                    break;
                }
            };
            if !s.trim().is_empty() {
                info!("{}", s);
            }
        }
        ()
    });

    let output_handler = Some(c.sender.clone());
    (default_fun, output_handler)
}