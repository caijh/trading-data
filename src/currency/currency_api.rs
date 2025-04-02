use std::error::Error;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde_json::Value;
use util::request::Request;

use crate::currency::currency_model::CurrencyRate;

pub async fn get_rate() -> Result<Vec<CurrencyRate>, Box<dyn Error>> {
    let response = Request::get_response("https://fx.cmbchina.com/api/v1/fx/rate").await?;
    let data: Value = response.json().await?;
    let return_code = data.get("returnCode").unwrap().as_str().unwrap_or_default();
    if return_code != "SUC0000" {
        return Err("获取汇率信息失败".into());
    }
    let body = data.get("body").unwrap().as_array();
    let mut currency_rates = Vec::new();
    if let Some(rates) = body {
        for rate in rates {
            let price = CurrencyRate {
                from: rate.get("ccyNbrEng").unwrap().as_str().unwrap().to_string(),
                to: "RMB".to_string(),
                buy_price: BigDecimal::from_str(rate.get("rthOfr").unwrap().as_str().unwrap())
                    .unwrap()
                    / 100, // 购汇
                sell_price: BigDecimal::from_str(rate.get("rthBid").unwrap().as_str().unwrap())
                    .unwrap()
                    / 100, // 结汇
            };
            currency_rates.push(price);
        }
    }
    Ok(currency_rates)
}
