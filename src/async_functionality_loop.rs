// use std::convert::Infallible;
// use std::future::Future;
// use std::time::Duration;
//
// use bytes::{Buf, Bytes};
// use futures::channel::oneshot;
// use serde::{Deserialize, Serialize};
// use serde::de::DeserializeOwned;
// use warp::Filter;
// use warp::http::StatusCode;
// use warp::Rejection;
// use warp::reply::Json;
//
// use crate::api::{Client, with_response_logger};
// use crate::schema::structs::ErrorInfo;
// use crate::util;
//
// /*
// See this for details
// https://blog.logrocket.com/create-an-async-crud-web-service-in-rust-with-warp/
//  */
// async fn warp_fn() {
//     let hello = warp::get().and_then(|| async move {
//         let res: Result<&str, warp::reject::Rejection> = Ok("hello");
//         res
//     });
//
//     let health_route = warp::path!("health").map(|| StatusCode::OK);
//
//     let routes = health_route.with(warp::cors().allow_any_origin());
//
//     warp::serve(routes.or(hello))
//         .run(([127, 0, 0, 1], 8001))
//         .await;
// }
//
// #[derive(Clone, Debug, Copy)]
// pub struct DBPool {
//     x: i64,
// }
//
// impl DBPool {
//
//     pub async fn health_handler4(&self, req: Req) -> std::result::Result<Json, Rejection> {
//         let string = format!("yello {} req {}", self.x.to_string(), req.x);
//         let resp = Resp { y: string };
//         log::info!("response from handler {}", serde_json::to_string(&resp.clone()).unwrap_or("failed to serialize response".into()));
//         Ok(warp::reply::json(&resp))
//     }
//
// }
//
// pub async fn health_handler(db_pool: DBPool) -> std::result::Result<&'static str, Rejection> {
//     Ok("yello")
// }
// fn with_db(db_pool: DBPool) -> impl Filter<Extract = (DBPool,), Error = Infallible> + Clone {
//     warp::any().map(move || db_pool.clone())
// }
//
// async fn warp_fn2(db_pool: DBPool) {
//     let hello = warp::get()
//         .and(with_db(db_pool.clone()))
//         .and_then(health_handler);
//     warp::serve(hello).run(([127, 0, 0, 1], 8002)).await;
// }
//
// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct Req {
//     x: String,
// }
//
// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct Resp {
//     y: String,
// }
//
// pub async fn health_handler3(db_pool: DBPool, req: Req) -> std::result::Result<String, Rejection> {
//     let string = format!("yello {} req {}", db_pool.x.to_string(), req.x);
//     Ok(string)
// }
//
// async fn warp_fn3(db_pool: DBPool) {
//     let hello = warp::post()
//         .and(with_db(db_pool.clone()))
//         .and(warp::body::json::<Req>())
//         .and_then(health_handler3);
//     warp::serve(hello).run(([127, 0, 0, 1], 8003)).await;
// }
//
// /*
//
// curl -0 -v -X POST http://localhost:8004/ \
// -H "Expect:" \
// -H 'Content-Type: application/json; charset=utf-8' \
// --data-binary @- << EOF
// {"x": "asdf"}
// EOF
//
// */
// //
// // fn log_body() -> impl Filter<Extract = (), Error = Rejection> + Copy {
// //     warp::body::bytes()
// //         .map(|b: Bytes| {
// //             println!("Request body: {}", std::str::from_utf8(b.bytes()).expect("error converting bytes to &str"));
// //         })
// //         .untuple_one()
// // }
//
// pub async fn health_handler4(db_pool: DBPool, req: Req) -> std::result::Result<Json, Rejection> {
//     let string = format!("yello {} req {}", db_pool.x.to_string(), req.x);
//     let resp = Resp { y: string };
//     log::info!("response from handler {}", serde_json::to_string(&resp.clone()).unwrap_or("failed to serialize response".into()));
//     Ok(warp::reply::json(&resp))
// }
//
// pub async fn health_handler_just_req(db_pool: DBPool, req: Req) -> Result<Resp, Rejection> {
//     let string = format!("yello {} req {}", db_pool.x.to_string(), req.x);
//     let resp = Resp { y: string };
//     // log::info!("response from handler {}", serde_json::to_string(&resp.clone()).unwrap_or("failed to serialize response".into()));
//     Ok(resp)
// }
// pub async fn health_handler_just_req_err_info(db_pool: DBPool, req: Req) -> Result<Resp, ErrorInfo> {
//     let string = format!("yello {} req {}", db_pool.x.to_string(), req.x);
//     let resp = Resp { y: string };
//     // log::info!("response from handler {}", serde_json::to_string(&resp.clone()).unwrap_or("failed to serialize response".into()));
//     Ok(resp)
// }
//
// pub async fn health_handler_format_json(resp: Resp) -> std::result::Result<Json, Rejection> {
//     // log::info!("response from handler {}", serde_json::to_string(&resp.clone()).unwrap_or("failed to serialize response".into()));
//     Ok(warp::reply::json(&resp))
// }
//
// fn log_body() -> impl Filter<Extract = (), Error = Rejection> + Copy {
//     warp::body::bytes()
//         .map(|b: Bytes| {
//             println!("Request body: {}", std::str::from_utf8(b.as_ref()).expect("error converting bytes to &str"));
//         })
//         .untuple_one()
// }
//
// /*
// use std::future::Future;
//
// async fn example<Fut>(f: impl FnOnce(i32, i32) -> Fut)
// where
//     Fut: Future<Output = bool>,
// {
//     f(1, 2).await;
// }
//
//  */
//
//
// fn warp_easy_post2<T, ReqT, RespT, Fut>(
//     clonable: T,
//     endpoint: String,
//     handler: fn(T, ReqT) -> Fut,
// ) -> impl Filter<Extract = (RespT,), Error = Rejection> + Clone
//     where T : Clone + Send,
//      ReqT : DeserializeOwned + Send + std::fmt::Debug,
//      RespT : DeserializeOwned + Send + std::fmt::Debug,
//       Fut: Future<Output = Result<RespT, Rejection>> + Send,
// {
//     warp::post()
//         .and(warp::path(endpoint.clone()))
//         .map(move || clonable.clone())
//         .and(warp::body::json::<ReqT>())
//         .map(move |x, y| {
//             log::debug!("Request {}: {:?} ", endpoint.clone(), y);
//             (x, y)
//         })
//         .untuple_one()
//         .and_then(handler)
// }
//
// fn warp_easy_post(db_pool: DBPool, endpoint: String) -> impl Filter<Extract = (Json,), Error = Rejection> + Clone
//  {
//      // let h = |x,y| health_handler4(x,y)
//      // warp_easy_post2(db_pool, endpoint.clone(), health_handler_just_req)
//      crate::api::easy_post(db_pool, endpoint.clone(), health_handler_just_req)
//          .map(move |r| with_response_logger(r, endpoint.clone()))
//         // .and_then(health_handler_just_req)
//         .and_then(health_handler_format_json)
// }
//
// async fn warp_fn4(db_pool: DBPool, endpoint: String) {
//     // let log = warp::log("redgold::api::fake_endpoint");
//     // let log = warp::log::custom(|info| {
//     //     // Use a log macro, or slog, or println, or whatever!
//     //     eprintln!(
//     //         "{} {} {} {}",
//     //         info.method(),
//     //         info.path(),
//     //         info.status(),
//     //         info.
//     //     );
//     // });
//     let hello = warp_easy_post(db_pool.clone(), endpoint.into());
//         // .with(log);
//     warp::serve(hello).run(([127, 0, 0, 1], 8004)).await;
// }
//
// struct ServiceInfo {
//     count: i64,
//     channel: oneshot::Receiver<i64>,
// }
//
// impl ServiceInfo {
//     // works with or without & ref
//     pub async fn service_one(&mut self) {
//         let mut interval = tokio::time::interval(Duration::from_secs(1));
//         loop {
//             interval.tick().await;
//             self.count += 1;
//             log::info!("yo service {}", self.count);
//         }
//     }
// }
//
// // This works without impl
// async fn service_one_works(mut info: ServiceInfo) {
//     let mut interval = tokio::time::interval(Duration::from_secs(1));
//     loop {
//         interval.tick().await;
//         info.count += 1;
//         log::info!("yo service {}", info.count);
//     }
// }
//
// async fn service_two() {
//     let mut interval = tokio::time::interval(Duration::from_secs(1));
//     loop {
//         interval.tick().await;
//         log::info!("yo service 2");
//     }
// }
//
// async fn req_test() -> Result<Resp, ErrorInfo> {
//     let client = Client::new("localhost".into(), 8004);
//     let req = Req {
//         x: "reeeequest".into(),
//     };
//     tokio::time::sleep(Duration::from_secs(2)).await;
//     client.json_post::<Req, Resp>(&req, "request".into()).await
// }
//
// #[tokio::test]
// async fn debug() {
//     // try with a runtime to see if behaves diff??
//     util::init_logger().ok();
//     let (s, r) = oneshot::channel();
//     let mut info = ServiceInfo {
//         count: 0,
//         channel: r,
//     };
//
//     let mut result: Resp = Resp { y: "failed".into() };
//     // warp_fn().await;
//     let db_pool = DBPool { x: 0 };
//
//     tokio::select! {
//         // _ = info.service_one() => {}
//         // _ = service_two() => {}
//         // _ = warp_fn() => {}
//         // _ = warp_fn2(db_pool.clone()) => {}
//         // _ = warp_fn3(db_pool.clone()) => {}
//         _ = warp_fn4(db_pool.clone(), "request".into()) => {}
//         res = req_test() => {
//             result = res.expect("propagate error later");
//             println!("yes");
//         }
//     }
//
//     let expected = Resp {
//         y: "yello 0 req reeeequest".to_string(),
//     };
//     assert_eq!(result, expected);
// }
