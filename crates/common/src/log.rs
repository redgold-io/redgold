use redgold_schema::conf::node_config::NodeConfig;
use std::sync::Once;
use crate::trace_setup::init_tracing;

pub fn init_logger_with_config(node_config: &NodeConfig) {
    // TODO: Log level from config
    init_tracing(&node_config.log_level);
}

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
pub fn init_logger_once() {
    INIT.call_once(|| {
        init_logger();
    });
}

pub fn init_logger_main(log_level: String) {
    INIT.call_once(|| {
        init_tracing(&log_level);
    });
}

pub fn init_logger() {
    // use tracing::LevelFilter;
    // use tracing4rs::append::console::ConsoleAppender;
    // use tracing4rs::append::file::FileAppender;
    // use tracing4rs::config::{Appender, Config, Root};
    //
    // let stdout = ConsoleAppender::builder().build();
    //
    // let root_redgold = FileAppender::builder()
    //     //.encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
    //     .build("../../log/redgold.log")
    //     .unwrap();
    //
    // let config = Config::builder()
    //     .appender(Appender::builder().build("stdout", Box::new(stdout)))
    //     .appender(Appender::builder().build("redgold", Box::new(root_redgold)))
    //     .logger(Logger::builder().build("sqlx", LevelFilter::Warn))
    //     .logger(Logger::builder().build("warp", LevelFilter::Warn))
    //     .logger(Logger::builder().build("rocket", LevelFilter::Warn))
    //     .logger(Logger::builder().build("redgold", LevelFilter::Debug))
    //     // .logger(Logger::builder().build("app::backend::db", LevelFilter::Info))
    //     // .logger(
    //     //     Logger::builder()
    //     //         .appender("requests")
    //     //         .additive(false)
    //     //         .build("app::requests", LevelFilter::Info),
    //     // )
    //     .build(
    //         Root::builder()
    //             .appenders(vec!["stdout", "redgold"])
    //             .build(LevelFilter::Info), //.appender("redgold").build(LevelFilter::Warn)
    //     )
    //     .unwrap();
    //
    // log4rs::init_config(config)
    init_tracing("DEBUG");
    // info!("Logger initialized");
}