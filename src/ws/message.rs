use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::LinkedList;
use std::io::Cursor;
use std::io::Read;
use thiserror::Error;

#[allow(non_camel_case_types)]
pub mod notification_msg {
    use serde::de::Error;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(tag = "cmd")]
    pub enum NotificationMsg {
        LIVE {},
        #[serde(rename = "DANMU_MSG:4:0:2:2:2:0")]
        DANMU_MSG_N {
            info: DanmuMsg,
        },
        DANMU_MSG {
            info: DanmuMsg,
        },
        ENTRY_EFFECT {
            data: EntryEffect,
        },
        ENTRY_EFFECT_MUST_RECEIVE {},
        INTERACT_WORD {
            data: Interact,
        },
        NOTICE_MSG {},
        STOP_LIVE_ROOM_LIST {},
        SEND_GIFT {
            data: OneGift,
        },
        COMBO_SEND {
            data: BatchGift,
        },
        GUARD_BUY {
            data: GuardBuy,
        },

        ROOM_BLOCK_MSG {},
        ROOM_CHANGE {},
        ROOM_REAL_TIME_MESSAGE_UPDATE {},
        HOT_RANK_CHANGED {},
        HOT_RANK_SETTLEMENT {},
        ONLINE_RANK_TOP3 {},
        ONLINE_RANK_COUNT {},
        ONLINE_RANK_V2 {},
        PK_BATTLE_PRE {},
        PK_BATTLE_START {},
        PK_BATTLE_END {},
        PK_BATTLE_SETTLE_USER {},
        PK_BATTLE_SETTLE_V2 {},
        PK_BATTLE_SETTLE {},
        PK_BATTLE_PRE_NEW {},
        PK_BATTLE_START_NEW {},
        PK_BATTLE_PROCESS_NEW {},
        PK_BATTLE_PROCESS {},
        WIDGET_BANNER {},
        COMMON_NOTICE_DANMAKU {},
        LITTLE_MESSAGE_BOX {},
        TRADING_SCORE {},
        WATCHED_CHANGE {},
        AREA_RANK_CHANGED {},
        LIKE_INFO_V3_UPDATE {},
        LIKE_INFO_V3_CLICK {},
    }

    #[derive(Serialize, Debug)]
    pub struct DanmuMsg {
        pub uid: u64,
        pub uname: String,

        pub medal_lv: u32,
        pub medal_name: String,
        pub medal_owner_uid: u64,
        pub medal_owner_name: String,

        pub text: String,
    }

    impl<'de> Deserialize<'de> for DanmuMsg {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let info = serde_json::Value::deserialize(deserializer)?;
            match info {
                Value::Array(ref info) => match info.as_slice() {
                    [_, Value::String(text), Value::Array(user), Value::Array(up), ..] => {
                        let uid = user.get(0).and_then(|v| v.as_u64()).unwrap_or(0);
                        let uname = user
                            .get(1)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let card_lv = up.get(0).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let card_name =
                            up.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let up_uid = up.last().and_then(|v| v.as_u64()).unwrap_or(0);
                        let up_name = up.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        Ok(DanmuMsg {
                            uid,
                            uname,
                            medal_lv: card_lv,
                            medal_name: card_name,
                            medal_owner_uid: up_uid,
                            medal_owner_name: up_name,
                            text: text.to_string(),
                        })
                    }
                    _ => Err(Error::custom("info format error")),
                },
                _ => Err(Error::custom("info type error")),
            }
        }
    }

    #[derive(Deserialize, Serialize, Default, Debug)]
    pub struct EntryEffect {
        #[serde(default)]
        pub uid: u64,
        #[serde(default)]
        pub copy_writing: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct Interact {
        #[serde(default)]
        pub uid: u64,
        #[serde(default)]
        pub uname: String,
        #[serde(default)]
        pub fans_medal: Medal,
        /**
        1. 进入直播间
        2. 关注直播间
        3. 分享直播间
        4. 未知
        5. 互关
        */
        pub msg_type: u32,
    }

    #[derive(Deserialize, Serialize, Default, Debug)]
    pub struct Medal {
        pub anchor_roomid: u32,
        pub guard_level: u32,
        pub medal_level: u32,
        pub medal_name: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct GuardBuy {
        pub gift_id: u32,
        pub gift_name: String,
        pub guard_level: u32,
        pub num: u32,
        pub uid: u64,
        pub username: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct OneGift {
        #[serde(rename = "giftId")]
        pub gift_id: u32,
        #[serde(rename = "giftName")]
        pub gift_name: String,
        pub total_coin: u32,
        pub num: u32,
        pub uid: u64,
        pub uname: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct BatchGift {
        pub gift_id: u32,
        pub gift_name: String,
        pub total_num: u32,
        pub combo_total_coin: u32,
        pub uid: u64,
        pub uname: String,
    }
}
#[derive(Debug)]
pub enum ServerLiveMessage {
    LoginAck,
    Notification(notification_msg::NotificationMsg),
    ServerHeartBeat,
}

#[derive(Debug, Clone)]
pub struct WsLogin {
    pub room_id: u32,
    pub uid: u32,
    pub key: String,
}

pub enum ClientLiveMessage {
    Login(WsLogin),
    ClientHeartBeat,
}

impl ClientLiveMessage {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ClientLiveMessage::Login(WsLogin { room_id, uid, key }) => {
                let uid = if *uid > 0 { Some(*uid) } else { None };
                let payload = serde_json::json!({
                        "uid": uid,
                        "roomid": *room_id,
                        "protover": 2,
                        "platform": "web",
                        "type": 2,
                        "key": key})
                .to_string();
                let payload_len = payload.len();
                let package_len = 16 + payload_len;

                let mut package = Vec::<u8>::with_capacity(package_len);
                package
                    .write_u32::<NetworkEndian>(package_len as u32)
                    .unwrap();
                package.write_u16::<NetworkEndian>(16).unwrap();
                package.write_u16::<NetworkEndian>(1).unwrap();
                package.write_u32::<NetworkEndian>(7).unwrap();
                package.write_u32::<NetworkEndian>(1).unwrap();
                package.extend_from_slice(payload.as_bytes());
                package
            }
            ClientLiveMessage::ClientHeartBeat => {
                let payload = b"[object Object]";
                let payload_len = payload.len();
                let package_len = 16 + payload_len;

                let mut package = Vec::<u8>::with_capacity(package_len);
                package.write_u32::<NetworkEndian>(16).unwrap();
                package.write_u16::<NetworkEndian>(16).unwrap();
                package.write_u16::<NetworkEndian>(1).unwrap();
                package.write_u32::<NetworkEndian>(2).unwrap();
                package.write_u32::<NetworkEndian>(1).unwrap();
                package.extend_from_slice(payload);
                package
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum MsgDecodeError {
    #[error("bad header")]
    BadHeader,
    #[error("useless msg:type = {0}")]
    UselessMsg(usize),
    #[error("inflate error {0}")]
    InflateError(String),
    #[error("undefine msg v={pkg_v:?} type={pkg_type:?}")]
    UndefinedMsg { pkg_v: u16, pkg_type: u32 },
    #[error("decode body is error {0}")]
    DecodeBodyError(String),
}

pub fn decode_from_server(
    data: Vec<u8>,
    result_list: &mut LinkedList<ServerLiveMessage>,
) -> Result<(), MsgDecodeError> {
    let mut buff_len = data.len();
    let mut buff = Cursor::new(data);
    'start: loop {
        let package_length = buff
            .read_u32::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)? as usize;
        let package_head_length = buff
            .read_u16::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)? as usize;
        let package_version = buff
            .read_u16::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;
        let package_type = buff
            .read_u32::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;
        let package_other = buff
            .read_u32::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;

        trace!(
            "package_version={} package_other={}",
            package_version,
            package_other
        );

        if package_version == 2 {
            let mut package_body = vec![];
            let _ = buff.read_to_end(&mut package_body);

            let new_data = inflate::inflate_bytes_zlib(package_body.as_slice())
                .map_err(|e| MsgDecodeError::InflateError(e))?;

            buff_len = new_data.len();
            buff = Cursor::new(new_data);
            // tail call
            continue 'start;
        }
        if package_version > 2 {
            return Err(MsgDecodeError::UndefinedMsg {
                pkg_v: package_version,
                pkg_type: package_type,
            });
        }

        let package_body_len = package_length - package_head_length;
        let mut package_body = vec![0; package_body_len];
        let _ = buff.read(package_body.as_mut_slice());

        match package_type {
            3 => result_list.push_back(ServerLiveMessage::ServerHeartBeat),
            5 => {
                let notification_msg = serde_json::from_slice(package_body.as_slice())
                    .map_err(|e| MsgDecodeError::DecodeBodyError(e.to_string()))?;
                result_list.push_back(ServerLiveMessage::Notification(notification_msg))
            }
            8 => result_list.push_back(ServerLiveMessage::LoginAck),
            _ => {
                return Err(MsgDecodeError::UndefinedMsg {
                    pkg_v: package_version,
                    pkg_type: package_type,
                });
            }
        };
        if buff.position() < buff_len as u64 {
            continue 'start;
        } else {
            break 'start;
        }
    }
    Ok(())
}
