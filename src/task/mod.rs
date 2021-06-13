use crate::bili_api::APIClient;
use crate::ws::{MsgStream, NotificationMsg, ServerLiveMessage};

pub async fn run(mut ws_client: MsgStream, api_client: APIClient) {
    while let Some(recv_msg) = ws_client.rx.recv().await {
        match recv_msg {
            ServerLiveMessage::LoginAck => {
                debug!("login ack")
            }
            ServerLiveMessage::Notification(notification) => match notification {
                NotificationMsg::LIVE { .. } => {
                    info!("直播开始");
                }
                NotificationMsg::DANMU_MSG { info: msg }
                | NotificationMsg::DANMU_MSG_N { info: msg } => {
                    info!("弹幕: {:?}", msg);
                }
                NotificationMsg::ENTRY_EFFECT { data } => {
                    info!("舰长进入直播间: {:?}", data);
                }
                NotificationMsg::INTERACT_WORD { data } => match data.msg_type {
                    1 => {
                        info!("进入直播间: {:?}", data);
                    }
                    2 => {
                        info!("关注直播间: {:?}", data);
                    }
                    3 => {
                        info!("分享直播间: {:?}", data);
                    }
                    5 => {
                        info!("互关: {:?}", data);
                    }
                    _ => {
                        info!("未知: {:?}", data);
                    }
                },
                NotificationMsg::ENTRY_EFFECT_MUST_RECEIVE { .. } => {}
                NotificationMsg::NOTICE_MSG { .. } => {}
                NotificationMsg::STOP_LIVE_ROOM_LIST { .. } => {}
                NotificationMsg::SEND_GIFT { data: gift } => {
                    info!("礼物: {:?}", gift);
                }
                NotificationMsg::COMBO_SEND { data: gift } => {
                    info!("礼物连击: {:?}", gift);
                }
                NotificationMsg::GUARD_BUY { data: guard_buy } => {
                    info!("购买大航海: {:?}", guard_buy);
                }
                _ => {}
            },
            ServerLiveMessage::ServerHeartBeat => {
                debug!("heart_beat")
            }
        }
    }
    warn!("ws client recv none,loop stop")
}
