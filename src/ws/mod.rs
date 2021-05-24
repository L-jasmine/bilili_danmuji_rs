pub mod message;

pub use crate::ws::message::notification_msg::NotificationMsg;
pub use crate::ws::message::{ClientLiveMessage, MsgDecodeError, ServerLiveMessage};
use anyhow::Error;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::collections::LinkedList;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

pub struct MsgStream {
    pub room_id: u32,
    pub rx: Receiver<ServerLiveMessage>,
    pub connect_handler: JoinHandle<Result<(), Error>>,
}

type WsStream = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type RsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const BILI_CHAT_SERVER_URL: &'static str = "wss://broadcastlv.chat.bilibili.com/sub";

pub async fn connect(room_id: u32) -> MsgStream {
    let url = BILI_CHAT_SERVER_URL.parse().unwrap();

    let (wx, rx) = tokio::sync::mpsc::channel(100);
    let connect_handler = tokio::spawn(open_client(url, room_id, wx));
    MsgStream {
        room_id,
        rx,
        connect_handler,
    }
}

pub async fn open_client(
    url: Url,
    room_id: u32,
    wx: Sender<ServerLiveMessage>,
) -> Result<(), Error> {
    loop {
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut w_stream, mut r_stream) = ws_stream.split();
        let r = tokio::join!(
            connect_keep(&mut w_stream, room_id),
            loop_handle_msg(&mut r_stream, wx.clone())
        );
        info!("client close {:?} {:?}", r.0, r.1);
        tokio::time::sleep(Duration::from_millis(500)).await;
        info!("reconnect start");
    }
}

async fn connect_keep(client: &mut WsStream, room_id: u32) -> Result<(), Error> {
    client
        .send(Message::Binary(
            ClientLiveMessage::Login { room_id }.encode(),
        ))
        .await
        .map_err(|e| anyhow!("{:?}", e))?;
    loop {
        info!("heartbeat");
        client
            .send(Message::Binary(ClientLiveMessage::ClientHeartBeat.encode()))
            .await
            .map_err(|e| anyhow!("{:?}", e))?;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn loop_handle_msg(
    client: &mut RsStream,
    wx: Sender<ServerLiveMessage>,
) -> Result<(), Error> {
    let mut msg_list = LinkedList::new();
    while let Some(msg) = client.next().await {
        let msg = msg?;
        match msg {
            Message::Text(text) => {
                debug!("recv text {}", text)
            }
            Message::Binary(bin) => {
                if let Err(e) = message::decode_from_server(bin, &mut msg_list) {
                    error!("handler msg {:?}", e)
                }
                while let Some(msg) = msg_list.pop_front() {
                    match msg {
                        ServerLiveMessage::LoginAck => {
                            debug!("LoginAck");
                        }
                        ServerLiveMessage::Notification(_) => {
                            debug!("Notification");
                        }
                        ServerLiveMessage::ServerHeartBeat => {
                            debug!("ServerHeartBeat");
                        }
                    }
                    wx.send(msg).await.map_err(|e| anyhow!("{:?}", e))?;
                }
            }
            Message::Ping(_) => debug!("ws ping"),
            Message::Pong(_) => debug!("ws pong"),
            Message::Close(_) => warn!("ws close"),
        }
    }
    warn!("ws handle loop stop");
    Ok(())
}

#[tokio::test]
async fn client_test() {
    env_logger::init();
    let mut s = connect(421296).await;

    while let Some(x) = s.rx.recv().await {
        match x {
            ServerLiveMessage::LoginAck => {
                info!("login ack")
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
                    info!("STOP_LIVE_ROOM_LIST");
                }
            },
            ServerLiveMessage::ServerHeartBeat => {
                info!("heart_beat")
            }
        }
    }
}

#[test]
fn qr_test() {
    use qrcode::render::unicode;
    use qrcode::QrCode;

    let code = QrCode::new(
        "https://passport.bilibili.com/qrcode/h5/login?oauthKey=beb978f21de4a6dbcba53c720e155560",
    )
    .unwrap();
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        // .quiet_zone(true)
        .build();
    println!("qrcode");
    println!("{}", image);
}
