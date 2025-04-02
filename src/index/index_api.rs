use crate::exchange::exchange_model::Exchange;
use crate::stock::stock_model::{Model as Stock, Model};
use async_trait::async_trait;
use calamine::Reader;
use calamine::Xls;
use calamine::open_workbook;
use rand::Rng;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use tempfile::tempdir;
use tracing::info;
use util::request::Request;

#[async_trait]
pub trait IndexApi {
    /// 获取指定指数包含的股票列表
    ///
    /// # Parameters
    ///
    /// * `index_code`: 指数代码，用于标识特定的指数
    ///
    /// # Returns
    ///
    /// 返回一个结果，包含股票列表（`Vec<Stock>`）或错误信息（`Box<dyn Error>`）
    async fn get_index_stocks(&self, index_code: &str) -> Result<Vec<Stock>, Box<dyn Error>>;
}

#[async_trait]
impl IndexApi for Exchange {
    async fn get_index_stocks(&self, index_code: &str) -> Result<Vec<Stock>, Box<dyn Error>> {
        match self {
            Exchange::SSE | Exchange::SZSE => {
                let url = format!(
                    "https://oss-ch.csindex.com.cn/static/html/csindex/public/uploads/file/autofile/cons/{}cons.xls",
                    index_code,
                );
                info!("Query Index Stocks from url = {}", url);
                let dir = tempdir()?;
                let path = dir.path().join(format!("{}cons.xls", index_code));
                download(&url, &path).await?;
                let stocks = read_index_stocks_from_excel(&path).await?;
                Ok(stocks)
            }
            Exchange::HKEX => get_index_stock_from_hkex(index_code, self).await,
            Exchange::NASDAQ => get_stocks_from_nasdaq(index_code, self).await,
        }
    }
}

async fn get_index_stock_from_hkex(
    index_code: &str,
    exchange: &Exchange,
) -> Result<Vec<Model>, Box<dyn Error>> {
    let url = format!(
        "https://www.hsi.com.hk/data/schi/rt/index-series/{}/constituents.do?{}",
        index_code,
        rand::rng().random_range(1000..9999)
    );
    info!("Query Index Stocks from url = {}", url);
    let response = Request::get_response(&url).await?;
    let data: Value = response.json().await?;
    let index_series_list = data.get("indexSeriesList").unwrap().as_array().unwrap();
    let index_series = index_series_list.first().unwrap().as_object().unwrap();
    let index_list = index_series.get("indexList").unwrap().as_array().unwrap();
    let index_stocks = index_list
        .first()
        .unwrap()
        .get("constituentContent")
        .unwrap()
        .as_array()
        .unwrap();
    let mut stocks = Vec::new();
    for index_stock in index_stocks {
        let code = index_stock
            .get("code")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let stock = Stock {
            code: format!("{}{}", code, exchange.stock_code_suffix()),
            name: index_stock
                .get("constituentName")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            exchange: "HK".to_string(),
            stock_type: "Stock".to_string(),
            stock_code: code,
        };
        stocks.push(stock);
    }
    Ok(stocks)
}

async fn get_stocks_from_nasdaq(
    _index: &str,
    exchange: &Exchange,
) -> Result<Vec<Stock>, Box<dyn Error>> {
    let url = format!("https://api.nasdaq.com/api/quote/list-type/{}", "nasdaq100");
    info!("Query Index Stocks from url = {}", url);
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
    let response = client.get(&url).headers(headers).send().await?;
    let text = response.text().await?;
    let data = serde_json::from_str::<Value>(&text).unwrap();
    let data = data.get("data").unwrap();
    let data = data.get("data").unwrap();
    let rows = data.get("rows").unwrap().as_array().unwrap();
    let mut stocks = Vec::new();
    for row in rows {
        let code = row.get("symbol").unwrap().as_str().unwrap().to_string();
        let mut name = row
            .get("companyName")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let idx = name.find(".");
        if idx.is_some() {
            name = name[..idx.unwrap()].to_string();
        }
        let stock = Stock {
            code: format!("{}{}", code, exchange.stock_code_suffix()),
            name,
            exchange: exchange.as_ref().to_string(),
            stock_type: "Stock".to_string(),
            stock_code: code,
        };
        stocks.push(stock);
    }
    Ok(stocks)
}

async fn download(url: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::builder().build().unwrap();
    let response = client.get(url).send().await;
    match response {
        Ok(response) => {
            let bytes = response.bytes().await?;
            let mut file = File::create(path)?;
            copy(&mut bytes.as_ref(), &mut file)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn read_index_stocks_from_excel(path: &Path) -> Result<Vec<Stock>, Box<dyn Error>> {
    let mut excel_xlsx: Xls<_> = open_workbook(path)?;

    let mut stocks = Vec::new();
    if let Some(Ok(result)) = excel_xlsx.worksheet_range_at(0) {
        for (i, row) in result.rows().enumerate() {
            if i == 0 {
                continue;
            }
            let exchange: Exchange = if row[7] == "深圳证券交易所" {
                Exchange::SZSE
            } else {
                Exchange::SSE
            };
            let stock_code = row[4].to_string();
            let stock_name = row[5].to_string();
            stocks.push(Stock {
                code: format!("{}{}", stock_code, exchange.stock_code_suffix()),
                name: stock_name,
                exchange: exchange.as_ref().to_string(),
                stock_type: "Stock".to_string(),
                stock_code,
            });
        }
    }

    Ok(stocks)
}
