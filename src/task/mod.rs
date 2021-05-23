use crate::bili_api::APIClient;
use crate::ws::{MsgStream, NotificationMsg, ServerLiveMessage};

pub async fn run(mut ws_client: MsgStream, api_client: APIClient) {
    while let Some(recv_msg) = ws_client.rx.recv().await {
        match recv_msg {
            ServerLiveMessage::LoginAck => {
                debug!("login ack")
            }
            ServerLiveMessage::Notification(notification) => match notification {
                NotificationMsg::DANMU_MSG { info: msg } => {
                    info!("弹幕: {:?}", msg);
                }
                NotificationMsg::ENTRY_EFFECT { data } => {
                    info!("舰长进入直播间: {:?}", data);
                }
                NotificationMsg::INTERACT_WORD { data } => {
                    info!("进入直播间: {:?}", data);
                }
                NotificationMsg::NOTICE_MSG {} => {
                    info!("NOTICE_MSG");
                }
                NotificationMsg::STOP_LIVE_ROOM_LIST {} => {
                    debug!("STOP_LIVE_ROOM_LIST");
                }
                NotificationMsg::SEND_GIFT { data: gift } => {
                    info!("礼物: {:?}", gift);
                }
                NotificationMsg::COMBO_SEND { data: gift } => {
                    info!("礼物连击: {:?}", gift);
                }
                NotificationMsg::ONLINE_RANK_COUNT {} => {
                    info!("ONLINE_RANK_COUNT");
                }
                NotificationMsg::ONLINE_RANK_V2 {} => {
                    info!("ONLINE_RANK_V2");
                }
                NotificationMsg::GUARD_BUY { data: guard_buy } => {
                    info!("购买大航海: {:?}", guard_buy);
                }
                NotificationMsg::ROOM_REAL_TIME_MESSAGE_UPDATE {} => {
                    debug!("ROOM_REAL_TIME_MESSAGE_UPDATE")
                }
            },
            ServerLiveMessage::ServerHeartBeat => {
                debug!("heart_beat")
            }
        }
    }
}
