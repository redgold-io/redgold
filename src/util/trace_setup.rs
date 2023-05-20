use crate::util;

use std::{error::Error, io};
use eframe::epaint::ahash::{HashMap, HashMapExt};
use tokio::task_local;
use tracing::{debug, error, info, span, warn, Level};
use redgold_schema::{EasyJson, error_info, error_message, task_local};
use redgold_schema::structs::ErrorInfo;

// the `#[tracing::instrument]` attribute creates and enters a span
// every time the instrumented function is called. The span is named after the
// the function or method. Parameters passed to the function are recorded as fields.
#[tracing::instrument]
pub fn shave(yak: usize, details: HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    // this creates an event at the DEBUG level with two fields:
    // - `excitement`, with the key "excitement" and the value "yay!"
    // - `message`, with the key "message" and the value "hello! I'm gonna shave a yak."
    //
    // unlike other fields, `message`'s shorthand initialization is just the string itself.
    info!("hello! I'm gonna shave a yak from within the instrument tracing function.");
    debug!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    if yak == 3 {
        warn!("could not locate yak!");
        // note that this is intended to demonstrate `tracing`'s features, not idiomatic
        // error handling! in a library or application, you should consider returning
        // a dedicated `YakError`. libraries like snafu or thiserror make this easy.
        return Err(io::Error::new(io::ErrorKind::Other, "shaving yak failed!").into());
    } else {
        debug!("yak shaved successfully");
    }
    Ok(())
}
//
// pub fn shave_all(yaks: usize) -> usize {
//     // Constructs a new span named "shaving_yaks" at the TRACE level,
//     // and a field whose key is "yaks". This is equivalent to writing:
//     //
//     // let span = span!(Level::TRACE, "shaving_yaks", yaks = yaks);
//     //
//     // local variables (`yaks`) can be used as field values
//     // without an assignment, similar to struct initializers.
//     let span = span!(Level::TRACE, "shaving_yaks", yaks);
//     let _enter = span.enter();
//
//     info!("shaving yaks");
//
//     let mut yaks_shaved = 0;
//     for yak in 1..=yaks {
//         let res = shave(yak);
//         debug!(yak, shaved = res.is_ok());
//
//         if let Err(ref error) = res {
//             // Like spans, events can also use the field initialization shorthand.
//             // In this instance, `yak` is the field being initialized.
//             error!(yak, error = error.as_ref(), "failed to shave yak!");
//         } else {
//             yaks_shaved += 1;
//         }
//         debug!(yaks_shaved);
//     }
//
//     yaks_shaved
// }


task_local! {
    pub static ONE: u32;

    #[allow(unused)]
    static TWO: f32;

    static NUMBER: u32;
}

async fn some_other_async() {
    println!("some other async number get: {}", NUMBER.get().to_string());
}

// #[tokio::test]
pub async fn debug() {

    //
    // NUMBER.scope(1, async move {
    //     // NUMBER.try_with()
    //     assert_eq!(NUMBER.get(), 1);
    // }).await;
    //
    // NUMBER.scope(2, async {
    //     println!("Number: {}", NUMBER.get().to_string());
    //     some_other_async().await;
    //     assert_eq!(NUMBER.get(), 2);
    //
    //     NUMBER.scope(3, async move {
    //         assert_eq!(NUMBER.get(), 3);
    //     }).await;
    // }).await;
    // util::init_logger();
    // let subscriber = tracing_subscriber::fmt()
    //     // filter spans/events with level TRACE or higher.
    //     .with_max_level(Level::TRACE)
    //     // build but do not install the subscriber.
    //     .finish();
    //
    // tracing::subscriber::with_default(subscriber, || {
    //     info!("This will be logged to stdout");
    // });
    // info!("This will _not_ be logged to stdout");
    // log::info!("This is log crate");
    // install global subscriber configured based on RUST_LOG envvar.


    tracing_subscriber::fmt::init();

    // let number_of_yaks = 3;
    // // this creates a new event, outside of any spans.
    // info!(number_of_yaks, "preparing to shave yaks");
    //
    // let number_shaved = shave_all(number_of_yaks);
    // info!(
    //     all_yaks_shaved = number_shaved == number_of_yaks,
    //     "yak shaving completed."
    // );

    let mut hm = HashMap::new();
    hm.insert("asdf".to_string(), "asdf".to_string());
    shave(0, hm);


}

pub fn init_tracing() {
    // TODO By environment and log level etc.
    tracing_subscriber::fmt::init();
}

async fn debug_task() -> ErrorInfo {
    error_info("yo")
}

#[tokio::test]
pub async fn debug_task_local() {
    let r = task_local("test", "asdf", debug_task()).await;
    println!("r: {}", r.json_pretty_or());
}