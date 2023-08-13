// #[cfg(test)]
// mod tests {
//     use log::LevelFilter;
//     use log::{debug, error, info, log_enabled, Level};
//     use log4rs::append::console::ConsoleAppender;
//     use log4rs::append::file::FileAppender;
//     use log4rs::config::{Appender, Config, Logger, Root};
//     use log4rs::encode::pattern::PatternEncoder;
//
//     fn init() {
//         let _ = env_logger::builder().is_test(true).try_init();
//     }
//
//     #[test]
//     fn it_works() {
//         let stdout = ConsoleAppender::builder().build();
//
//         let root_redgold = FileAppender::builder()
//             //.encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
//             .build("../log/redgold.log")
//             .unwrap();
//
//         let config = Config::builder()
//             .appender(Appender::builder().build("stdout", Box::new(stdout)))
//             .appender(Appender::builder().build("redgold", Box::new(root_redgold)))
//             // .logger(Logger::builder().build("app::backend::db", LevelFilter::Info))
//             // .logger(
//             //     Logger::builder()
//             //         .appender("requests")
//             //         .additive(false)
//             //         .build("app::requests", LevelFilter::Info),
//             // )
//             .build(
//                 Root::builder()
//                     .appenders(vec!["stdout", "redgold"])
//                     .build(LevelFilter::Warn), //.appender("redgold").build(LevelFilter::Warn)
//             )
//             .unwrap();
//
//         let handle = log4rs::init_config(config).unwrap();
//
//         info!("This record will be captured by `cargo test`");
//         debug!("this is a debug {}", "message");
//         error!("this is printed by default");
//
//         assert_eq!(2, 1 + 1);
//     }
// }
