pub mod message;

pub use crate::ws::message::notification_msg::NotificationMsg;
pub use crate::ws::message::{ClientLiveMessage, MsgDecodeError, ServerLiveMessage, WsLogin};
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
    pub ws_login: WsLogin,
    pub rx: Receiver<ServerLiveMessage>,
    pub connect_handler: JoinHandle<Result<(), Error>>,
}

type WsStream = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type RsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const BILI_CHAT_SERVER_URL: &'static str = "wss://broadcastlv.chat.bilibili.com/sub";

pub async fn connect(ws_login: WsLogin) -> MsgStream {
    let url = BILI_CHAT_SERVER_URL.parse().unwrap();

    let (wx, rx) = tokio::sync::mpsc::channel(100);
    let connect_handler = tokio::spawn(open_client(url, ws_login.clone(), wx));
    MsgStream {
        ws_login,
        rx,
        connect_handler,
    }
}

pub async fn open_client(
    url: Url,
    ws_login: WsLogin,
    wx: Sender<ServerLiveMessage>,
) -> Result<(), Error> {
    let mut reconnect_time = 0;
    loop {
        let start_time = std::time::SystemTime::now();
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut w_stream, mut r_stream) = ws_stream.split();
        let r = tokio::join!(
            connect_keep(&mut w_stream, ws_login.clone()),
            loop_handle_msg(&mut r_stream, wx.clone())
        );
        info!("client close {:?} {:?}", r.0, r.1);
        let now = std::time::SystemTime::now();
        let d = now.duration_since(start_time).unwrap().as_secs();
        if d > (60 * 30) {
            reconnect_time = 0;
        }
        let time = if reconnect_time <= 20 { 10 } else { 300 };
        info!("reconnect[{}] after {} secs", reconnect_time, time);
        tokio::time::sleep(Duration::from_secs(time)).await;
        info!("reconnect start");
    }
}

async fn connect_keep(client: &mut WsStream, ws_login: WsLogin) -> Result<(), Error> {
    client
        .send(Message::Binary(ClientLiveMessage::Login(ws_login).encode()))
        .await
        .map_err(|e| anyhow!("{:?}", e))?;
    loop {
        debug!("heartbeat");
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
    let mut s = connect(WsLogin {
        room_id: 421296,
        uid: 0,
        key: "".to_string(),
    })
    .await;

    while let Some(x) = s.rx.recv().await {
        info!("{:?}", x);
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
