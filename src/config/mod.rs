use serde::{Deserialize, Serialize};

lazy_static! {
    pub static ref APP_CONFIG: AppConfig = init_config();
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AppConfig {
    pub room_id: u32,
}

pub fn init_config() -> AppConfig {
    let log_str = std::fs::read_to_string("config.json").expect("no config.json");
    serde_json::from_str(log_str.as_str()).unwrap()
}

pub fn logger_config() {
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
                .build(log::LevelFilter::Debug),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
