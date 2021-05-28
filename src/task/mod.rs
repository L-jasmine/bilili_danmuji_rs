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
                NotificationMsg::ROOM_BLOCK_MSG { .. } => {}
                NotificationMsg::ROOM_REAL_TIME_MESSAGE_UPDATE { .. } => {}
                NotificationMsg::HOT_RANK_CHANGED { .. } => {}
                NotificationMsg::ONLINE_RANK_TOP3 { .. } => {}
                NotificationMsg::ONLINE_RANK_COUNT { .. } => {}
                NotificationMsg::ONLINE_RANK_V2 { .. } => {}
                NotificationMsg::PK_BATTLE_END { .. } => {}
                NotificationMsg::PK_BATTLE_SETTLE_USER { .. } => {}
                NotificationMsg::PK_BATTLE_SETTLE_V2 { .. } => {}
                NotificationMsg::PK_BATTLE_SETTLE { .. } => {}
                NotificationMsg::PK_BATTLE_PRE_NEW { .. } => {}
                NotificationMsg::PK_BATTLE_START_NEW { .. } => {}
                NotificationMsg::PK_BATTLE_PROCESS_NEW { .. } => {}
                NotificationMsg::PK_BATTLE_PROCESS { .. } => {}
            },
            ServerLiveMessage::ServerHeartBeat => {
                debug!("heart_beat")
            }
        }
    }
    warn!("ws client recv none,loop stop")
}
