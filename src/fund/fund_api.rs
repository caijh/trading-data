use crate::exchange::exchange_model::Exchange;
use crate::stock::stock_model::{Model as Stock, StockKind};
use async_trait::async_trait;
use calamine::{Reader, Xlsx, open_workbook};
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
            Exchange::SZSE => {
                let url = format!(
                    "http://www.szse.cn/api/report/ShowReport?SHOWTYPE=xlsx&CATALOGID=1105&TABKEY=tab1&random={}",
                    rng().random::<f64>()
                );
                let dir = tempdir()?;
                let path_buf = dir.path().join("sz_funds.xlsx");
                Request::download(&url, path_buf.as_path()).await?;
                let stocks = read_funds_from_sz_excel(path_buf.as_path(), self)?;
                Ok(stocks)
            }
            Exchange::HKEX => Ok(vec![
                Stock {
                    code: "2800.HK".to_string(),
                    name: "盈富基金".to_string(),
                    exchange: "HKEX".to_string(),
                    stock_type: "Fund".to_string(),
                    stock_code: "2800".to_string(),
                },
                Stock {
                    code: "7300.HK".to_string(),
                    name: "恒生一倍看空".to_string(),
                    exchange: "HKEX".to_string(),
                    stock_type: "Fund".to_string(),
                    stock_code: "7300".to_string(),
                },
            ]),
            Exchange::NASDAQ => get_funds_from_nasdaq(self).await,
        }
    }
}

async fn get_funds_from_nasdaq(exchange: &Exchange) -> Result<Vec<Stock>, Box<dyn Error>> {
    let url =
        "https://api.nasdaq.com/api/screener/etf?download=true&region=north-america".to_string();
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
