use anyhow::Error;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::{ACCEPT, ORIGIN, REFERER, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

const BILI_URL: &'static str = "https://api.bilibili.com";
const TOKEN_PATH: &'static str = "./token";

const COOKIE_USER_ID: &'static str = "DedeUserID=";
const COOKIE_SESSDATA: &'static str = "SESSDATA=";
const COOKIE_BILI_JCT: &'static str = "bili_jct=";

const UA: &'static str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/81.0.4044.138 Safari/537.36";

#[derive(Debug, Clone)]
pub struct UserToken {
    pub uid: String,
    pub token: String,
    pub csrf: String,
}

#[derive(Debug, Clone)]
pub struct APIClient {
    pub client: Client,
    pub token: UserToken,
}

fn check_cookie(jar: &Jar) -> Result<UserToken, Error> {
    let domain_url = BILI_URL.parse().unwrap();
    let cookies = jar
        .cookies(&domain_url)
        .ok_or(anyhow!("cookies is empty"))?;

    let cookies = cookies.to_str().map_err(|e| anyhow!("{}", e))?;

    let mut token = UserToken {
        uid: "".to_string(),
        token: "".to_string(),
        csrf: "".to_string(),
    };

    for c in cookies.split(";") {
        let c = c.trim();
        if c.starts_with(COOKIE_USER_ID) {
            let (_, v) = c.split_at(COOKIE_USER_ID.len());
            token.uid = v.to_string();
        } else if c.starts_with(COOKIE_SESSDATA) {
            let (_, v) = c.split_at(COOKIE_SESSDATA.len());
            token.token = v.to_string();
        } else if c.starts_with(COOKIE_BILI_JCT) {
            let (_, v) = c.split_at(COOKIE_SESSDATA.len());
            token.csrf = v.to_string();
        } else {
            info!("cookie {}", c)
        }
    }

    if token.uid.is_empty() || token.token.is_empty() || token.csrf.is_empty() {
        Err(anyhow!("check_cookie error {:?}", cookies))
    } else {
        Ok(token)
    }
}

fn get_client_from_file() -> Result<APIClient, Error> {
    info!("get token from file `token`");
    let domain_url = BILI_URL.parse().unwrap();
    let jar = Jar::default();
    let tokens = std::fs::read_to_string(TOKEN_PATH).map_err(|e| anyhow!("{}", e))?;
    let tokens = tokens.split('\n');
    for cookie in tokens {
        jar.add_cookie_str(cookie, &domain_url);
    }
    let token = check_cookie(&jar)?;
    let client = Client::builder()
        .cookie_provider(Arc::new(jar))
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| anyhow!("{}", e))?;
    Ok(APIClient { client, token })
}

#[tokio::test]
async fn test_get_client_from_bili() {
    let r = get_client_from_bili().await.unwrap();
    println!("{}", r.token.uid)
}

async fn get_client_from_bili() -> Result<APIClient, Error> {
    let login_url = get_login_url().await?;

    if let Some(ref url) = login_url.data {
        print_login_qrcode(url.url.as_str());

        'check: loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let (client, login_result) = get_bili_client(url.qrcode_key.as_str()).await?;
            println!("{:?}", login_result);
            if login_result.code == 0 {
                if let Some(r) = login_result.data {
                    if r.code != 0 {
                        println!("{}", r.message);
                        continue 'check;
                    } else {
                        return Ok(client);
                    }
                }
            } else {
                return Err(anyhow!("get login url error \n {:?}", login_result));
            }
        }
    }
    Err(anyhow!("get login url error \n {:?}", login_url))
}

pub async fn get_client() -> Result<APIClient, Error> {
    info!("get_client_from_file");
    let maybe_client = get_client_from_file();
    match maybe_client {
        Ok(client) => Ok(client),
        Err(e) => {
            warn!("get_client_from_file {:?}", e);
            info!("get_client_from_bili");
            get_client_from_bili().await
        }
    }
}

#[tokio::test]
async fn test_get_client() {
    println!("abc");
    env_logger::init();
    let r = get_client().await;
    println!("{:?}", r);
}

// api

#[derive(Deserialize, Serialize, Debug)]
pub struct APIResult<T> {
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub ttl: u32,
    #[serde(default)]
    pub ts: u32,
    pub data: Option<T>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginUrl {
    pub url: String,
    pub qrcode_key: String,
}

#[tokio::test]
async fn test_get_login_url() {
    let login_url = get_login_url().await.unwrap();
    println!("{:?}", login_url);
}

pub async fn get_login_url() -> Result<APIResult<LoginUrl>, Error> {
    // https://passport.bilibili.com/x/passport-login/web/qrcode/generate?source=main-fe-header
    let resp = reqwest::get(
        "https://passport.bilibili.com/x/passport-login/web/qrcode/generate?source=main-fe-header",
    )
    .await
    .map_err(|e| anyhow!("request {:?}", e))?;
    let r = resp
        .json::<APIResult<LoginUrl>>()
        .await
        .map_err(|e| anyhow!("parse {:?}", e))?;
    return Ok(r);
}

pub fn print_login_qrcode(login_url: &str) {
    use qrcode::render::svg;
    use qrcode::QrCode;

    let code = QrCode::new(login_url).unwrap();

    {
        let image = code
            .render::<char>()
            .light_color('#')
            .dark_color(' ')
            .module_dimensions(2, 1)
            .build();
        println!("{}\n===【 手机app扫描上方二维码登陆 】===", image,);
    }
    {
        println!("===【{} {} {}】===", "或者双击打开", "qr.svg", "扫码登陆");
        let image = code
            .render()
            .min_dimensions(200, 200)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();
        std::fs::write("qr.svg", image.as_str()).unwrap();
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct QrResult {
    url: String,
    refresh_token: String,
    timestamp: u64,
    code: i32,
    message: String,
}

pub async fn get_bili_client(qrcode_key: &str) -> Result<(APIClient, APIResult<QrResult>), Error> {
    info!("get_bili_client by {}", qrcode_key);
    // https://passport.biligame.com/x/passport-login/web/crossDomain?DedeUserID=16856350&DedeUserID__ckMd5=59f1d8365143ac66&Expires=1720765201&SESSDATA=56773412,1720765201,36615*12CjDbRK4JVBD6u2H_LA9a3C2px9CKaaCVwidTnrfjLYSIJc0PisIZZE2VRrNZMToXbIUSVngzNWo1b1loOFM1T25yVEMzZHM0bnRCdGdpWjVoeEhvVTRYME41MW5FMVFTRlpVbm9aWm1ZU2NTR2hkYndOeF9idTc3UlNVRFN6M2xDbml2ZV9ya1RBIIEC&bili_jct=6080ad32f93a316bbb1fc71cd30c6cd5&gourl=https%3A%2F%2Fwww.bilibili.com

    let jar = Arc::new(Jar::default());

    let client = Client::builder()
        .cookie_provider(jar.clone())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| anyhow!("{}", e))?;

    let form_param = [("qrcode_key", qrcode_key), ("source", "main-fe-header")];
    let resp = client
        .get(format!("https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}&source=main-fe-header", qrcode_key))
        .header(USER_AGENT, UA)
        .header(ACCEPT, "application/json, text/plain, */*")
        .header(REFERER, "https://www.bilibili.com")
        .header(ORIGIN, "https://www.bilibili.com")
        .form(&form_param)
        .send()
        .await
        .map_err(|e| anyhow!("reqwest qrcode/poll error {}", e))?;

    let header_cookies = resp.headers().get_all("set-cookie");
    let mut cookies = String::new();

    for cookie_value in header_cookies {
        match cookie_value.to_str() {
            Ok(cookie) => {
                cookies.push_str(cookie);
                cookies.push('\n');
            }
            Err(e) => {
                error!("login cookie to str error : {:?}", e)
            }
        }
    }
    if cookies.ends_with("\n") {
        cookies.pop();
    }

    let r = resp
        .json::<APIResult<QrResult>>()
        .await
        .map_err(|e| anyhow!("parse qrcode/poll respone error : {}", e))?;

    let token = if r.code == 0 && r.data.is_some() && r.data.as_ref().unwrap().code == 0 {
        let token = check_cookie(jar.as_ref())?;
        //save token
        info!("save token");
        std::fs::write(TOKEN_PATH, cookies)
            .map_err(|e| anyhow!("{}", e))
            .unwrap();

        token
    } else {
        UserToken {
            uid: "".to_string(),
            token: "".to_string(),
            csrf: "".to_string(),
        }
    };

    Ok((APIClient { client, token }, r))
}

pub async fn send_barrage(
    api_client: &APIClient,
    room_id: &str,
    barrage: &str,
) -> Result<APIResult<serde_json::Value>, Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    let now = format!("{}", now.as_secs());
    let param = [
        ("color", "16777215"), // 默认白色
        ("fontsize", "25"),
        ("mode", "1"), // 1 是滚动弹幕 4 是底部弹幕
        ("msg", barrage),
        ("rnd", now.as_str()),
        ("roomid", room_id),
        ("bubble", "0"),
        ("csrf_token", api_client.token.csrf.as_str()),
        ("csrf", api_client.token.csrf.as_str()),
    ];
    let resp = api_client
        .client
        .post("https://api.live.bilibili.com/msg/send")
        .header(USER_AGENT, UA)
        .header(reqwest::header::REFERER, "https://live.bilibili.com")
        .form(&param)
        .send()
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let r = resp
        .json::<APIResult<serde_json::Value>>()
        .await
        .map_err(|e| anyhow!("{}", e))?;
    Ok(r)
}

#[tokio::test]
async fn test_send_barrage() {
    let client = get_client().await.unwrap();
    let r = send_barrage(&client, "421296", "弹幕测试").await;
    println!("{:?}", r)
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum BanUserResult {
    Success { uname: String },
    Fail(Vec<()>),
}

pub async fn ban_user(
    api_client: &APIClient,
    room_id: &str,
    block_uid: &str,
    hour: u32,
) -> Result<APIResult<BanUserResult>, Error> {
    let hour = format!("{}", hour);
    let param = [
        ("roomid", room_id),
        ("block_uid", block_uid),
        ("hour", hour.as_str()),
        ("csrf_token", api_client.token.csrf.as_str()),
        ("csrf", api_client.token.csrf.as_str()),
        ("visit_id", ""),
    ];
    let resp = api_client
        .client
        .post("https://api.live.bilibili.com/banned_service/v2/Silent/add_block_user")
        .header(USER_AGENT, UA)
        .header(reqwest::header::REFERER, "https://live.bilibili.com")
        .form(&param)
        .send()
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let r = resp
        .json::<APIResult<BanUserResult>>()
        .await
        .map_err(|e| anyhow!("{}", e))?;
    Ok(r)
}

#[tokio::test]
async fn test_ban_user() {
    let client = get_client().await.unwrap();
    let r = ban_user(&client, "421296", "386121455", 1).await;
    println!("{:?}", r);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let r = ban_user(&client, "421295", "386121455", 1).await;
    println!("{:?}", r)
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FollowUser {
    pub mid: u32,
    pub uname: String,
    pub mtime: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FollowResult {
    pub list: Vec<FollowUser>,
    pub total: u32,
}

pub async fn get_some_followings(
    api_client: &APIClient,
    uid: &str,
    page: u32,
    page_size: u32,
) -> Result<APIResult<FollowResult>, Error> {
    let resp = api_client
        .client
        .get(format!(
            "https://api.bilibili.com/x/relation/same/followings?vmid={}&ps={}&pn={}",
            uid, page_size, page
        ))
        .header(USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let r = resp
        .json::<APIResult<FollowResult>>()
        .await
        .map_err(|e| anyhow!("{}", e))?;
    Ok(r)
}

#[tokio::test]
async fn test_get_some_followings() {
    let client = get_client().await.unwrap();
    let r = get_some_followings(&client, "2", 1, 50).await;
    println!("{:?}", r);
}

pub async fn search_followings(
    api_client: &APIClient,
    uid: u32,
    name: &str,
    page: u32,
    page_size: u32,
) -> Result<APIResult<FollowResult>, Error> {
    let resp = api_client
        .client
        .get(format!(
            "https://api.bilibili.com/x/relation/followings/search?vmid={}&name={}&ps={}&pn={}",
            uid, name, page_size, page
        ))
        .header(USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let r = resp
        .json::<APIResult<FollowResult>>()
        .await
        .map_err(|e| anyhow!("{}", e))?;
    Ok(r)
}

#[tokio::test]
async fn test_search_followings() {
    let client = get_client().await.unwrap();
    let r = search_followings(&client, 2, "咬人猫", 1, 50).await;
    if let Ok(APIResult { data: Some(x), .. }) = &r {
        println!("{:?}", x);
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DanmuInfoResult {
    #[serde(default)]
    pub business_id: u32,
    #[serde(default)]
    pub host_list: Vec<LiveHost>,
    #[serde(default)]
    pub max_delay: u32,
    #[serde(default)]
    pub refresh_rate: u32,
    #[serde(default)]
    pub refresh_row_factor: f32,
    #[serde(default)]
    pub token: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LiveHost {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u32,
    #[serde(default)]
    pub ws_port: u32,
    #[serde(default)]
    pub wss_port: u32,
}

pub async fn get_danmu_info(
    api_client: &APIClient,
    room_id: u32,
) -> Result<APIResult<DanmuInfoResult>, Error> {
    let resp = api_client
        .client
        .get(format!(
            "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id={}&type=0",
            room_id
        ))
        .header(USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let r = resp
        .json::<APIResult<DanmuInfoResult>>()
        .await
        .map_err(|e| anyhow!("{}", e))?;
    Ok(r)
}
