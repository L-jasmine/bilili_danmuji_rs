#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

pub mod bili_api;
pub mod config;
pub mod task;
pub mod ws;

#[tokio::main]
async fn main() {
    // config::logger_config();
    env_logger::init();
    let room_id = config::APP_CONFIG.room_id;
    let api_client = bili_api::get_client().await.unwrap();
    let ws_client = ws::connect(api_client.clone(), room_id).await;
    task::run(ws_client, api_client).await;

    info!("exit")
}
