#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

use std::env::args;

pub mod bili_api;
pub mod config;
pub mod task;
pub mod ws;

#[tokio::main]
async fn main() {
    config::logger_config();
    let room_id = config::APP_CONFIG.room_id;
    let api_client = bili_api::get_client().await.unwrap();
    let danmu_info = bili_api::get_danmu_info(&api_client, room_id).await;

    if let Ok(bili_api::APIResult {
        data: Some(bili_api::DanmuInfoResult {
            host_list, token, ..
        }),
        ..
    }) = danmu_info
    {
        let uid = api_client.token.uid.parse().unwrap();
        let ws_client = ws::connect(ws::WsLogin {
            room_id,
            uid,
            key: token,
        })
        .await;

        task::run(ws_client, api_client).await;
    } else {
        error!("get danmu info error {:?}", danmu_info);
    }

    info!("exit")
}
