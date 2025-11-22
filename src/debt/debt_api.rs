use crate::debt::debt_model::DebtPrice;
use crate::exchange::exchange_model::Exchange;
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use async_trait::async_trait;
use chrono::{Local, NaiveDateTime};
use serde_json::Value;
use std::error::Error;
use util::request::Request;

#[async_trait]
pub trait DebtApi {
    async fn get_debt_price(&self, code: &str) -> Result<DebtPrice, Box<dyn Error>>;
}

#[async_trait]
impl DebtApi for Exchange {
    async fn get_debt_price(&self, code: &str) -> Result<DebtPrice, Box<dyn Error>> {
        match self {
            Exchange::SSE => get_debt_price(code).await,
            _ => Err("暂不支持该交易所".into()),
        }
    }
}
pub async fn get_debt_price(code: &str) -> Result<DebtPrice, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let client = Request::client().await;
    let url = environment
        .get_property::<String>("stock.api.sh.baseurl")
        .unwrap();
    let response = client
        .get(format!(
            "{}/v1/shb1/snap/{}?_={}",
            url,
            code,
            Local::now().timestamp_millis()
        ))
        .send()
        .await?;
    let json: Value = response.json().await?;
    let snap = json.get("snap").unwrap();
    let date = json.get("date").unwrap().to_string();
    let time = json.get("time").unwrap().to_string();
    let time = if time.len() < 6 {
        format!("{}{}", 0, time)
    } else {
        time
    };
    Ok(DebtPrice {
        yc: snap.get(1).unwrap().to_string(),
        open: snap.get(2).unwrap().to_string(),
        high: snap.get(3).unwrap().to_string(),
        low: snap.get(4).unwrap().to_string(),
        current: snap.get(5).unwrap().to_string(),
        zd: snap.get(6).unwrap().to_string(),
        zdf: snap.get(7).unwrap().to_string(),
        v: snap.get(8).unwrap().to_string(),
        cje: snap.get(9).unwrap().to_string(),
        t: NaiveDateTime::parse_from_str(&format!("{}{}", date, time), "%Y%m%d%H%M%S")
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    })
}
