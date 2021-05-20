use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::Write;
use std::io::{BufRead, Cursor};
use std::io::{Read, Seek};
use thiserror::Error;

#[derive(Debug)]
pub enum ServerLiveMessage {
    LoginAck,
    Notification(String),
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
    #[error("body is not utf8 string")]
    NotUtf8Body,
}

pub fn decode_from_server(data: Vec<u8>) -> Result<ServerLiveMessage, MsgDecodeError> {
    let mut buff = Cursor::new(data);
    'start: loop {
        let package_length = buff
            .read_u32::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;
        let package_head_length = buff
            .read_u16::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;
        let package_version = buff
            .read_u16::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;
        let package_type = buff
            .read_u32::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;
        let package_other = buff
            .read_u32::<NetworkEndian>()
            .map_err(|_| MsgDecodeError::BadHeader)?;

        let mut package_body = vec![];
        buff.read_to_end(&mut package_body);

        if package_version == 2 {
            let new_data = inflate::inflate_bytes_zlib(package_body.as_slice())
                .map_err(|e| MsgDecodeError::InflateError(e))?;
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

        return match package_type {
            3 => Ok(ServerLiveMessage::ServerHeartBeat),
            5 => Ok(ServerLiveMessage::Notification(
                String::from_utf8(package_body).map_err(|e| MsgDecodeError::NotUtf8Body)?,
            )),
            8 => Ok(ServerLiveMessage::LoginAck),
            _ => Err(MsgDecodeError::UndefinedMsg {
                pkg_v: package_version,
                pkg_type: package_type,
            }),
        };
    }
}

#[test]
fn test_msg_decode() {
    let mut v = Vec::<u8>::with_capacity(10);
    v.write_i8(1);
    v.write_i8(2);
    println!("{:?}", v)
}
