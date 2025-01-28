use crate::structs::ErrorDetails;
use itertools::Itertools;
use std::collections::HashMap;
use std::future::Future;
use tokio::task::futures::TaskLocalFuture;
use tokio::task_local;

// TODO: This feature is only available in tokio RT, need to substitute this for a
// standard local key implementation depending on the features available for WASM crate.
task_local! {

    pub static TASK_LOCAL: HashMap<String, String>;
    // pub static TASK_LOCAL: String;
    // pub static ONE: u32;
    //
    // #[allow(unused)]
    // static TWO: f32;
    //
    // static NUMBER: u32;
}


pub fn get_task_local() -> HashMap<String, String> {
    TASK_LOCAL.try_with(|local| { local.clone() })
        .unwrap_or(HashMap::new())
}

pub fn task_local<K: Into<String>, V: Into<String>, F>(k: K, v: V, f: F) -> TaskLocalFuture<HashMap<String, String>, F>
where F : Future{
    let mut current = get_task_local();
    current.insert(k.into(), v.into());
    TASK_LOCAL.scope(current, f)
}

pub fn task_local_map<F>(kv: HashMap<String, String>, f: F) -> TaskLocalFuture<HashMap<String, String>, F>
where F : Future{
    let mut current = get_task_local();
    for (k, v) in kv {
        current.insert(k, v);
    }
    TASK_LOCAL.scope(current, f)
}


pub fn task_local_error_details() -> Vec<ErrorDetails> {
    get_task_local().iter().map(|(k, v)| {
        ErrorDetails {
            detail_name: k.clone(),
            detail: v.clone(),
        }
    }).collect_vec()
}