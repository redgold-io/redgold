// use redgold::util;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing(log_level: &str) {

    use tracing_subscriber::EnvFilter;

    let fmt_layer = tracing_subscriber::fmt::Layer::default()
        .compact()
        .with_ansi(false);

    let filter_layer = EnvFilter::new(format!(
        "sqlx=ERROR,warp=WARN,rocket=ERROR,redgold={}", log_level));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}
