use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::LinkedList;
use std::io::Write;
use std::io::{BufRead, Cursor};
use std::io::{Read, Seek};
use thiserror::Error;

pub mod notification_msg {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize};
    use serde_json::Value;

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(tag = "cmd")]
    pub enum NotificationMsg {
        DANMU_MSG { info: DanmuMsg },
        ENTRY_EFFECT { data: User },
        INTERACT_WORD { data: User },
        NOTICE_MSG {},
        STOP_LIVE_ROOM_LIST {},
        SEND_GIFT { data: OneGift },
        COMBO_SEND { data: BatchGift },
        ONLINE_RANK_COUNT {},
        ONLINE_RANK_V2 {},
        GUARD_BUY { data: GuardBuy },
        ROOM_REAL_TIME_MESSAGE_UPDATE {},
    }

    #[derive(Serialize, Debug)]
    pub struct DanmuMsg {
        pub uid: u32,
        pub uname: String,

        pub card_lv: u32,
        pub card_name: String,
        pub card_owner_uid: u32,
        pub card_owner_name: String,

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
                        let uid = user.get(0).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let uname = user
                            .get(1)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let card_lv = up.get(0).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let card_name =
                            up.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let up_uid = up.last().and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let up_name = up.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        Ok(DanmuMsg {
                            uid,
                            uname,
                            card_lv,
                            card_name,
                            card_owner_uid: up_uid,
                            card_owner_name: up_name,
                            text: text.to_string(),
                        })
                    }
                    _ => Err(Error::custom("info format error")),
                },
                _ => Err(Error::custom("info type error")),
            }
        }
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct User {
        pub uid: u32,
        #[serde(default)]
        pub uname: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct GuardBuy {
        pub gift_id: u32,
        pub gift_name: String,
        pub guard_level: u32,
        pub num: u32,
        pub uid: u32,
        pub username: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct OneGift {
        #[serde(rename = "giftId")]
        pub gift_id: u32,
        #[serde(rename = "giftName")]
        pub gift_name: String,
        pub num: u32,
        pub uid: u32,
        pub uname: String,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct BatchGift {
        pub gift_id: u32,
        pub gift_name: String,
        pub total_num: u32,
        pub uid: u32,
        pub uname: String,
    }
}

#[derive(Debug)]
pub enum ServerLiveMessage {
    LoginAck,
    Notification(notification_msg::NotificationMsg),
    ServerHeartBeat,
}

pub enum ClientLiveMessage {
    Login { room_id: u32 },
    ClientHeartBeat,
}

impl ClientLiveMessage {
    pub fn encode(self) -> Vec<u8> {
        match self {
            ClientLiveMessage::Login { room_id } => {
                let payload = serde_json::json!({ "roomid": room_id }).to_string();
                let payload_len = payload.len();
                let package_len = 16 + payload_len;

                let mut package = Vec::<u8>::with_capacity(package_len);
                package.write_u32::<NetworkEndian>(package_len as u32);
                package.write_u16::<NetworkEndian>(16);
                package.write_u16::<NetworkEndian>(1);
                package.write_u32::<NetworkEndian>(7);
                package.write_u32::<NetworkEndian>(1);
                package.extend_from_slice(payload.as_bytes());
                package
            }
            ClientLiveMessage::ClientHeartBeat => {
                let mut package = Vec::<u8>::with_capacity(16);
                package.write_u32::<NetworkEndian>(16);
                package.write_u16::<NetworkEndian>(16);
                package.write_u16::<NetworkEndian>(1);
                package.write_u32::<NetworkEndian>(2);
                package.write_u32::<NetworkEndian>(1);
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

        if package_version == 2 {
            let mut package_body = vec![];
            buff.read_to_end(&mut package_body);

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
        buff.read(package_body.as_mut_slice());

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
                })
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
