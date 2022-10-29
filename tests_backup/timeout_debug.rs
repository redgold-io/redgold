use log::info;
use std::thread::sleep;
use std::time::Duration;
use tokio::time::timeout;

async fn long_func() {
    tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
}

use futures::pin_mut;
use redgold::util::init_logger;

async fn delay() {
    for _ in 0..6 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        info!("Ping!");
    }
}

async fn runner() {
    let delayer = delay();
    pin_mut!(delayer);
    if let Err(_) = tokio::time::timeout(std::time::Duration::from_secs(2), &mut delayer).await {
        info!("Taking more than two seconds");
        delayer.await;
    }
}
//
// #[tokio::test]
// async fn timeout_debug() {
//     init_logger();
//
//     runner().await;
//     // let res = timeout(Duration::from_secs(1), long_func()).await;
//     // assert!(res.is_err());
// }

#[tokio::test]
async fn timeout_debug2() {
    init_logger();
    let res = timeout(Duration::from_secs(1), long_func()).await;
    assert!(res.is_err());
}
