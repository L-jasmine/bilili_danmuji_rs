#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;

use std::env::args;

pub mod bili_api;
pub mod task;
pub mod ws;

fn logger_config() {
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Logger, Root};
    use log4rs::encode::pattern::PatternEncoder;

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "[{d(%Y-%m-%d %H:%M:%S)}] [{l}] {M} - {m} {n}",
        )))
        .build();
    let file_log = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "[{d(%Y-%m-%d %H:%M:%S)}] [{l}] {M} - {m} {n}",
        )))
        .build("log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file_log", Box::new(file_log)))
        .logger(
            Logger::builder()
                .appender("file_log")
                .build("bilili_danmuji_rs", log::LevelFilter::Info),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Info),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}

#[tokio::main]
async fn main() {
    logger_config();
    let room_id_str = args().nth(1).expect("no set room_id");
    let room_id = room_id_str.parse().expect("room_id no number");

    let api_client = bili_api::get_client().await.unwrap();
    let ws_client = ws::connect(room_id).await;

    task::run(ws_client, api_client).await;
}
