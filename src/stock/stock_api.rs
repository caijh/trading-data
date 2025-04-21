use crate::exchange::exchange_model::Exchange;
use crate::index::index_api::IndexApi;
use crate::stock::stock_model::Model;
use async_trait::async_trait;
use calamine::{Reader, Xls, Xlsx, open_workbook};
use rand::{Rng, rng};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use tempfile::tempdir;
use util::request::Request;

#[async_trait]
pub trait StockApi {
    async fn get_stocks(&self) -> Result<Vec<Model>, Box<dyn Error>>;
}

#[async_trait]
impl StockApi for Exchange {
    async fn get_stocks(&self) -> Result<Vec<Model>, Box<dyn Error>> {
        match self {
            Exchange::SSE => {
                let dir = tempdir()?;
                let url = "http://query.sse.com.cn/sseQuery/commonExcelDd.do?sqlId=COMMON_SSE_CP_GPJCTPZ_GPLB_GP_L&type=inParams&CSRC_CODE=&STOCK_CODE=&REG_PROVINCE=&STOCK_TYPE=1,8&COMPANY_STATUS=2,4,5,7,8";
                let path = dir.path().join("sh_stocks.xls");
                download(url, path.as_path()).await?;
                let stocks = read_stocks_from_sh_excel(path.as_path(), self)?;
                Ok(stocks)
            }
            Exchange::SZSE => {
                let dir = tempdir()?;
                let url = format!(
                    "https://www.szse.cn/api/report/ShowReport?SHOWTYPE=xlsx&CATALOGID=1110&TABKEY=tab1&random={}",
                    rng().random::<f64>()
                );
                let path = dir.path().join("sz_stocks.xlsx");
                Request::download(&url, path.as_path()).await?;
                let stocks = read_stocks_from_sz_excel(path.as_path(), self)?;
                Ok(stocks)
            }
            Exchange::HKEX => get_stock_from_hk().await,
            Exchange::NASDAQ => self.get_index_stocks("nasdaq100").await,
        }
    }
}

async fn get_stock_from_hk() -> Result<Vec<Model>, Box<dyn Error>> {
    let url = format!(
        "https://www.hsi.com.hk/data/schi/rt/index-series/hsi/constituents.do?{}",
        rng().random_range(1000..9999)
    );
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
        let stock = Model {
            code: format!("{}{}", code, Exchange::HKEX.stock_code_suffix()),
            name: index_stock
                .get("constituentName")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            exchange: Exchange::HKEX.as_ref().to_string(),
            stock_type: "Stock".to_string(),
            stock_code: code,
        };
        stocks.push(stock);
    }
    Ok(stocks)
}

pub async fn download(url: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("X-Requested-With", "XMLHttpRequest".parse().unwrap());
    headers.insert(
        "Referer",
        "http://www.sse.com.cn/assortment/stock/list/share/"
            .parse()
            .unwrap(),
    );
    headers.insert("Connection", "keep-alive".parse().unwrap());
    let client = reqwest::Client::builder().build().unwrap();
    let response = client.get(url).headers(headers).send().await;
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

pub fn read_stocks_from_sh_excel(
    path: &Path,
    exchange: &Exchange,
) -> Result<Vec<Model>, Box<dyn Error>> {
    let mut excel_xls: Xls<_> = open_workbook(path)?;

    let mut stocks = Vec::new();
    if let Ok(r) = excel_xls.worksheet_range("股票") {
        for row in r.rows() {
            if row[0] == "A股代码" {
                // 跳过标题行
                continue;
            }
            stocks.push(Model {
                code: format!("{}{}", row[0], exchange.stock_code_suffix()),
                name: row[2].to_string(),
                exchange: exchange.as_ref().to_string(),
                stock_type: "Stock".to_string(),
                stock_code: row[0].to_string(),
            });
        }
    }

    Ok(stocks)
}
pub fn read_stocks_from_sz_excel(
    path: &Path,
    exchange: &Exchange,
) -> Result<Vec<Model>, Box<dyn Error>> {
    let mut excel_xlsx: Xlsx<_> = open_workbook(path)?;

    let mut stocks = Vec::new();
    if let Ok(r) = excel_xlsx.worksheet_range("A股列表") {
        for row in r.rows() {
            if row[0] == "板块" {
                // 跳过标题行
                continue;
            }
            stocks.push(Model {
                code: format!("{}{}", row[4], exchange.stock_code_suffix()),
                name: row[5].to_string(),
                exchange: exchange.as_ref().to_string(),
                stock_type: "Stock".to_string(),
                stock_code: row[4].to_string(),
            });
        }
    }

    Ok(stocks)
}
