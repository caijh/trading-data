use crate::exchange::exchange_model::Exchange;
use crate::stock::stock_model::{Model as Stock, Model, StockKind};
use crate::token::token_svc;
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use async_trait::async_trait;
use calamine::{Reader, Xlsx, open_workbook};
use chrono::Local;
use rand::{Rng, rng};
use serde_json::Value;
use std::error::Error;
use std::path::Path;
use tempfile::tempdir;
use util::request::Request;

#[async_trait]
pub trait FundApi {
    async fn get_funds(&self) -> Result<Vec<Stock>, Box<dyn Error>>;
}

#[async_trait]
impl FundApi for Exchange {
    async fn get_funds(&self) -> Result<Vec<Stock>, Box<dyn Error>> {
        match self {
            Exchange::SSE => get_funds_from_sse(self).await,
            Exchange::SZSE => get_funds_from_szse(self).await,
            Exchange::HKEX => get_funds_from_hkex(self).await,
            Exchange::NASDAQ => get_funds_from_nasdaq(self).await,
        }
    }
}

async fn get_funds_from_szse(exchange: &Exchange) -> Result<Vec<Model>, Box<dyn Error>> {
    let url = format!(
        "http://www.szse.cn/api/report/ShowReport?SHOWTYPE=xlsx&CATALOGID=1105&TABKEY=tab1&random={}",
        rng().random::<f64>()
    );
    let dir = tempdir()?;
    let path_buf = dir.path().join("sz_funds.xlsx");
    Request::download(&url, path_buf.as_path()).await?;
    let stocks = read_funds_from_sz_excel(path_buf.as_path(), exchange)?;
    Ok(stocks)
}

async fn get_funds_from_hkex(exchange: &Exchange) -> Result<Vec<Stock>, Box<dyn Error>> {
    let subcat_list = vec!["7", "9"]; // 交易所买卖基金，反向基金
    let mut funds = Vec::new();
    for subcat in subcat_list {
        let stocks = get_funds_from_hkex_subcat(exchange, subcat).await?;
        funds.extend(stocks);
    }
    Ok(funds)
}

async fn get_funds_from_hkex_subcat(
    exchange: &Exchange,
    subcat: &str,
) -> Result<Vec<Stock>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.hk.baseurl")
        .unwrap();
    let token = token_svc::get_hkex_token().await;
    let timestamp = Local::now().timestamp_millis();
    let url = format!(
        "{}/hkexwidget/data/getetpfilter?lang=chi&token={}&subcat={}&sort=2&order=0&all=1&qid={}&callback=jQuery_{}&_={}",
        base_url, token, subcat, timestamp, timestamp, timestamp,
    );
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    let json = crate::stock::stock_price_api::remove_jquery_wrapping_fn_call(&text);
    let data = json.get("data").unwrap();
    let data = data.get("stocklist").unwrap().as_array();
    let data = data.unwrap();
    let mut funds = Vec::new();
    for stock in data {
        let code = stock.get("sym").unwrap().as_str().unwrap();
        funds.push(Stock {
            code: format!("{}{}", code, exchange.stock_code_suffix()),
            name: stock.get("nm").unwrap().as_str().unwrap().to_string(),
            exchange: exchange.as_ref().to_string(),
            stock_type: StockKind::Fund.to_string(),
            stock_code: code.to_string(),
        })
    }
    Ok(funds)
}

async fn get_funds_from_nasdaq(exchange: &Exchange) -> Result<Vec<Stock>, Box<dyn Error>> {
    let url = "https://api.nasdaq.com/api/screener/etf?download=true&assetclass=equity".to_string();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("Accept", "*/*".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();
    let response = client.get(url).headers(headers).send().await;
    match response {
        Ok(response) => {
            let json: Value = response.json().await?;
            let data = json
                .get("data")
                .unwrap()
                .get("data")
                .unwrap()
                .get("rows")
                .unwrap()
                .as_array();
            let mut funds = Vec::new();
            if let Some(data) = data {
                for fund in data {
                    let symbol = fund.get("symbol").unwrap().as_str().unwrap();
                    funds.push(Stock {
                        code: format!("{}{}", symbol, exchange.stock_code_suffix()),
                        name: symbol.to_string(),
                        exchange: exchange.as_ref().to_string(),
                        stock_type: StockKind::Fund.to_string(),
                        stock_code: symbol.to_string(),
                    });
                }
            }
            Ok(funds)
        }
        Err(e) => Err(e.into()),
    }
}

async fn get_funds_from_sse(exchange: &Exchange) -> Result<Vec<Stock>, Box<dyn Error>> {
    let url = format!(
        "https://query.sse.com.cn/commonSoaQuery.do?sqlId=FUND_LIST&fundType=00&_={}",
        rng().random::<f64>()
    );
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("X-Requested-With", "XMLHttpRequest".parse().unwrap());
    headers.insert("Referer", "https://www.sse.com.cn/".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    let client = reqwest::Client::builder().build().unwrap();
    let response = client.get(url).headers(headers).send().await;
    match response {
        Ok(response) => {
            let json: Value = response.json().await?;
            let data = json
                .get("pageHelp")
                .unwrap()
                .get("data")
                .unwrap()
                .as_array();
            let mut funds = Vec::new();
            if let Some(data) = data {
                for fund in data {
                    let stock = Stock {
                        code: format!(
                            "{}{}",
                            fund.get("fundCode").unwrap().as_str().unwrap().to_string(),
                            exchange.stock_code_suffix()
                        ),
                        name: fund
                            .get("secNameFull")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string(),
                        exchange: exchange.as_ref().to_string(),
                        stock_type: "Fund".to_string(),
                        stock_code: fund.get("fundCode").unwrap().as_str().unwrap().to_string(),
                    };
                    funds.push(stock);
                }
            }
            Ok(funds)
        }
        Err(e) => Err(e.into()),
    }
}

fn read_funds_from_sz_excel(
    path: &Path,
    exchange: &Exchange,
) -> Result<Vec<Stock>, Box<dyn Error>> {
    let mut excel_xlsx: Xlsx<_> = open_workbook(path)?;

    let mut stocks = Vec::new();
    if let Ok(r) = excel_xlsx.worksheet_range("基金列表") {
        for row in r.rows() {
            if row[0] == "基金代码" {
                // 跳过标题行
                continue;
            }
            stocks.push(Stock {
                code: format!("{}{}", row[0], exchange.stock_code_suffix()),
                name: row[1].to_string(),
                exchange: exchange.as_ref().to_string(),
                stock_type: "Fund".to_string(),
                stock_code: row[0].to_string(),
            });
        }
    }

    Ok(stocks)
}
