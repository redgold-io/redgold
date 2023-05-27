use std::thread::sleep;
use std::time::Duration;
use async_std::prelude::FutureExt;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::task::JoinError;
use redgold_schema::structs::ErrorInfo;
use crate::util;
use crate::util::runtimes::build_runtime;


struct ServiceInfo {
    count: i64,
    break_count: i64,
}

impl ServiceInfo {
    // works with or without & ref
    pub async fn service_one(&mut self) -> Result<(), ErrorInfo> {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            self.count += 1;
            log::info!("service 1 count {}", self.count);
            if self.count > self.break_count {
                log::info!("Breaking! {}", self.count);
                None::<Option<String>>.expect("true");
            }
        }
        // Err(ErrorInfo::error_info("service 1 error"))
    }
}

fn returns_err() -> Result<(), ErrorInfo> {
    return Err(ErrorInfo::error_info("error"));
}

fn start_func() {

    //
    // tokio::select! {
    //
    // }

    // returns_err().expect("fail");

}

#[ignore]
#[test]
fn start() {
    util::init_logger().ok();

    let rt = build_runtime(5, "test");
    let jh = rt.spawn(async {
        let mut info = ServiceInfo {
            count: 0,
            break_count: 2
        };
        info.service_one().await
    });
    let jh2 = rt.spawn(async {
        let mut info = ServiceInfo {
            count: 10,
            break_count: 15
        };
        info.service_one().await
    });
    // let mut jh3 = rt.spawn(async {
    //     let mut info = ServiceInfo {
    //         count: 20,
    //         break_count: 10000
    //     };
    //     info.service_one().await
    // });

    // let both =  async {
    //     tokio::select! {
    //     res = &mut jh => {
    //             res
    //     },
    //     res2 = &mut jh2 => {
    //             res2
    //     }
    //     }
    // };
    let mut futures = FuturesUnordered::new();
    futures.push(jh);
    futures.push(jh2);
    use futures::{stream::FuturesUnordered, StreamExt};
    let both = futures.next();


    let res: Option<Result<Result<(), ErrorInfo>, JoinError>> = rt.block_on(both);
    //
    // jh.abort();
    // jh2.abort();
    log::info!("Result {:?}", res);
    sleep(Duration::from_secs(1200));

}