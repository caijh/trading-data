use lazy_static::lazy_static;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use util::request::Request;

lazy_static! {
    static ref HKEX_TOKEN: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
}
pub async fn get_hkex_token_from_website() -> Result<String, Box<dyn Error>> {
    let res = Request::get_content("https://www.hkex.com.hk/Market-Data/Securities-Prices/Equities/Equities-Quote?sym=700&sc_lang=zh-HK").await?;
    let idx = res.find("\"Base64-AES-Encrypted-Token\";").unwrap();
    let token = &res[idx..];
    let idx = token.find("return").unwrap();
    let token = &token[idx..];
    let begin = token.find("\"").unwrap();
    let token = &token[begin..];
    let end = token.find(";").unwrap();
    let token = &token[0..end];
    let token = token.replace("\"", "");
    Ok(token.to_string())
}

pub async fn set_hkex_token(token: &str) {
    let mut hkex_token = HKEX_TOKEN.write().await;
    *hkex_token = token.to_string();
}

pub async fn reset_hkex_token() -> Result<(), Box<dyn Error>> {
    let token = get_hkex_token_from_website().await?;
    info!("token = {}", token);
    set_hkex_token(&token).await;

    Ok(())
}

pub async fn get_hkex_token() -> String {
    let hkex_token = HKEX_TOKEN.read().await;
    hkex_token.clone()
}
