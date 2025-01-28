use redgold_schema::structs::ErrorInfo;
use redgold_schema::util::lang_util::SameResult;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use warp::reply::Json;
use warp::{Filter, Rejection};

pub fn easy_post<T, ReqT, RespT, Fut, S>(
    clonable: T,
    endpoint: S,
    handler: fn(T, ReqT) -> Fut,
    length_limit_kb: u64
) -> impl Filter<Extract = (Result<RespT, ErrorInfo>,), Error = Rejection> + Clone
    where T : Clone + Send,
          ReqT : DeserializeOwned + Send + std::fmt::Debug + ?Sized + Serialize + Clone,
          RespT : DeserializeOwned + Send + std::fmt::Debug,
          Fut: Future<Output = Result<Result<RespT, ErrorInfo>, Rejection>> + Send,
          S: Into<String> + Clone + Send
{
    warp::post()
        .and(warp::path !("account" / "balance"))
        // .and(with_path_inner(endpoint.clone().into()))
        .and(warp::body::content_length_limit(1024 * length_limit_kb))
        .map(move || clonable.clone())
        .and(warp::body::json::<ReqT>())
        .map(move |x, y: ReqT| {
            let y2 = y.clone();
            let ser = serde_json::to_string(&y2).unwrap_or("request ser failed".to_string());
            log::debug!("Request endpoint: {} {} ", endpoint.clone().into(), ser);
            (x, y)
        })
        .untuple_one()
        .and_then(handler)
}

pub fn with_response_logger<Resp>(resp: Resp, endpoint: String) -> Resp
where Resp: ?Sized + Serialize + Clone {
    let y2 = resp.clone();
    let ser = serde_json::to_string(&y2).unwrap_or("response ser failed".to_string());
    log::debug!("Response endpoint: {} {} ", endpoint.clone(), ser);
    resp
}

pub fn with_response_logger_error<Resp, ErrT>(resp: Result<Resp, ErrT>, endpoint_i: String) -> Result<Resp, ErrT>
where Resp: ?Sized + Serialize + Clone,
ErrT: ?Sized + Serialize + Clone {
    let endpoint = endpoint_i.clone();
    let endpoint2 = endpoint_i.clone();
    resp.map(move |r| {
    let y2 = r.clone();
    let ser = serde_json::to_string( & y2).unwrap_or("response ser failed".to_string());
    log::debug ! ("Response success endpoint: {} {} ", endpoint.clone(), ser);
    r
    }).map_err(move |r| {
    let y2 = r.clone();
    let ser = serde_json::to_string( & y2).unwrap_or("response ser failed".to_string());
    log::debug ! ("Response error endpoint: {} {} ", endpoint2.clone(), ser);
    r
    })
}

// TODO: implement as trait on result
pub fn as_warp_json_response<T: Serialize, E: Serialize>(response: Result<T, E>) -> Result<Json, warp::reject::Rejection> {
    Ok(response.map_err(|e| warp::reply::json(&e))
    .map(|r| warp::reply::json(&r))
    .combine()
    )
}

//
// pub fn endpoint<T, C, R>(
//     path: &str,
//     c: C,
//     f: fn((C, T)) -> dyn Future<Output=Result<R, ErrorInfo>>,
//     fmt: fn(Result<R, ErrorInfo>) -> Result<Json, warp::reject::Rejection>,
// ) -> ()
// where
//     T: DeserializeOwned + Send + Sized,
//     C: Clone + Sized,
//     R: Serialize + Sized,
// {
//     let post = warp::post()
//         .and(warp::path(path))
//         // Only accept bodies smaller than 16kb...
//         .and(warp::body::content_length_limit(1024 * 16))
//         .and(warp::body::json::<T>())
//         .and_then(move |request: T| {
//             let cc = c.clone();
//             async move {
//                 let result: Result<R, ErrorInfo> = f((cc, request)).await;
//                 fmt(result)
//             }
//         });
//     post;
//
// }
//
// // TODO: Gotta be a better way to do this.
// pub fn with_path_inner1(
//     p: Vec<String>
// ) -> impl Filter<Extract = (), Error = Rejection> + Clone + Filter {
//     warp::any()
//         .and(warp::path(p.get(0).expect("0")))
// }
//
// pub fn with_path_inner2(
//     p: Vec<String>
// ) -> impl Filter<Extract = (), Error = Rejection> + Clone + Filter {
//     warp::log::custom()
//     warp::any()
//         .and(warp::path(p.get(0).expect("0")))
//         .and(warp::path(p.get(1).expect("0")))
// }
//
// pub fn with_path_inner(
//     endpoint: String
// ) -> impl Filter<Extract = (), Error = Rejection> + Clone + Filter {
//     let p = endpoint.split("/").collect_vec().iter().map(|s| s.to_string())
//         .collect_vec();
//     match p.len() {
//         1 => {
//             with_path_inner1(p.clone())
//         },
//         2 => {
//             with_path_inner2(p.clone())
//         },
//         _ => {panic!("Bad endpoint path length! {}", endpoint)}
//     }
// }


//
// pub fn with_response_logger<Resp>(resp: Resp, endpoint: String) -> impl Filter<Extract = (Resp,), Error = Infallible> + Clone
// where Resp: ?Sized + Serialize + Clone,{
//     warp::any().map(move |resp: Resp| {
//         let y2 = resp.clone();
//         let ser = serde_json::to_string(&y2).unwrap_or("response ser failed".into());
//         log::debug!("Response {}: {:?} ", endpoint.clone(), ser);
//         resp
//     })
// }
