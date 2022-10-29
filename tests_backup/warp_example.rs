// use async_std::task::spawn;
// use std::thread::sleep;
// use std::time::Duration;
// use warp::Filter;
//
// async fn request() -> Result<String, Box<dyn std::error::Error>> {
//     let resp = reqwest::get("http://127.0.0.1:13030/hello/wo")
//         .await?
//         .text()
//         .await?;
//     // println!("{:#?}", resp);
//     Ok(resp)
// }
//
// #[tokio::test]
// async fn test_warp_basic() {
//     let handle = tokio::task::spawn(run_server());
//     //handle.await;
//     sleep(Duration::new(1000, 0));
//     // let res = request().await.unwrap();
//     // assert_eq!("Hello, wo!", res);
//     // // println!("wut {:?}", request().await);
// }
//
// async fn run_server() {
//     // GET /hello/warp => 200 OK with body "Hello, warp!"
//     let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));
//
//     warp::serve(hello).run(([127, 0, 0, 1], 3030));
// }
mod bdk_example;
mod borrow_ref_tests;
mod sign_multisig;
