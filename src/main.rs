#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;

use std::env::args;

pub mod bili_api;
pub mod task;
pub mod ws;

#[tokio::main]
async fn main() {
    let room_id_str = args().nth(1).expect("no set room_id");
    let room_id = room_id_str.parse().expect("room_id no number");
    env_logger::init();

    let api_client = bili_api::get_client().await.unwrap();
    let ws_client = ws::connect(room_id).await;

    task::run(ws_client, api_client).await;
}
