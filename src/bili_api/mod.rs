use anyhow::Error;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::ToStrError;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

const BILI_URL: &'static str = "https://api.bilibili.com";
const TOKEN_PATH: &'static str = "token";

const COOKIE_USER_ID: &'static str = "DedeUserID=";
const COOKIE_SESSDATA: &'static str = "SESSDATA=";
const COOKIE_BILI_JCT: &'static str = "bili_jct=";

#[derive(Debug)]
pub struct UserToken {
    pub uid: String,
    pub token: String,
    pub csrf: String,
}

#[derive(Debug)]
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
    info!("get token.bk from file `token.bk`");
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

async fn get_client_from_bili() -> Result<APIClient, Error> {
    let login_url = get_login_url().await?;
    if let Some(ref url) = login_url.data {
        'check: loop {
            print_login_qrcode(url.url.as_str());
            println!("\n== 扫码确认后按回车 ==");
            let mut ignore = String::new();
            std::io::stdin().read_line(&mut ignore);

            let (client, login_result) = get_bili_client(url.oauth_key.as_str()).await?;
            if login_result.status {
                return Ok(client);
            } else if let Some(serde_json::Value::Number(n)) = login_result.data {
                if n == serde_json::Number::from(-4i64) {
                    println!("\n== 未扫描二维码 ==");
                    continue 'check;
                } else {
                    return Err(anyhow!("get login url error \n {:?}", login_result.message));
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

#[derive(Deserialize, Serialize, Debug)]
pub struct APIResult<T> {
    #[serde(default)]
    pub status: bool,
    #[serde(default)]
    pub code: u32,
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
    #[serde(rename = "oauthKey")]
    pub oauth_key: String,
}

pub async fn get_login_url() -> Result<APIResult<LoginUrl>, Error> {
    let resp = reqwest::get("https://passport.bilibili.com/qrcode/getLoginUrl")
        .await
        .map_err(|e| anyhow!("request {:?}", e))?;
    let r = resp
        .json::<APIResult<LoginUrl>>()
        .await
        .map_err(|e| anyhow!("parse {:?}", e))?;
    return Ok(r);
}

pub fn print_login_qrcode(login_url: &str) {
    use qrcode::render::unicode;
    use qrcode::QrCode;

    let code = QrCode::new(login_url).unwrap();
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        // .quiet_zone(true)
        .build();
    println!("===【 手机app扫码登陆 】===\n\n{}", image);
}

pub async fn get_bili_client(
    oauth_key: &str,
) -> Result<(APIClient, APIResult<serde_json::Value>), Error> {
    let jar = Arc::new(Jar::default());

    let client = Client::builder()
        .cookie_provider(jar.clone())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| anyhow!("{}", e))?;

    let form_param = [("oauthKey", oauth_key)];
    let resp = client
        .post("https://passport.bilibili.com/qrcode/getLoginInfo")
        .form(&form_param)
        .send()
        .await
        .map_err(|e| anyhow!("{}", e))?;

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
        .json::<APIResult<serde_json::Value>>()
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let token = if r.status {
        let token = check_cookie(jar.as_ref())?;
        //save token.bk
        std::fs::write(TOKEN_PATH, cookies).map_err(|e| anyhow!("{}", e));

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
