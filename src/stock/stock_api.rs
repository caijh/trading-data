use crate::exchange::exchange_model::Exchange;
use crate::index::index_api::IndexApi;
use crate::stock::stock_dao;
use crate::stock::stock_model::{Model, StockKind};
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use async_trait::async_trait;
use calamine::{Reader, Xls, Xlsx, open_workbook};
use rand::{RngExt, rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use std::str::FromStr;
use tempfile::tempdir;
use tracing::info;
use util::request::Request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpperLimitStock {
    pub serial_no: u32,
    pub stock_code: String,
    pub stock_name: String,
    pub change_percent: f64,
    pub latest_price: f64,
    pub turnover_amount: f64,
    pub circulating_market_cap: f64,
    pub total_market_cap: f64,
    pub turnover_rate: f64,
    pub limit_up_order_fund: f64,
    pub first_limit_up_time: String,
    pub last_limit_up_time: String,
    pub limit_up_break_count: u32,
    pub limit_up_stats: String,
    pub consecutive_limit_up_days: u32,
    pub industry: String,
}

#[async_trait]
pub trait StockApi {
    async fn get_stocks(&self) -> Result<Vec<Model>, Box<dyn Error>>;
    async fn get_upper_limit_stocks(&self, exchange: &str) -> Result<Vec<UpperLimitStock>, Box<dyn Error>>;
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
                let path1 = path.as_path();
                let stocks = read_stocks_from_excel(path1, self, "股票", 0, 2)?;
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
                let path1 = path.as_path();
                let stocks = read_stocks_from_excel(path1, self, "A 股列表", 4, 5)?;
                Ok(stocks)
            }
            Exchange::HKEX => get_stock_from_hk().await,
            Exchange::NASDAQ => {
                let mut stocks = Vec::new();
                let nasdaq100_index_stocks = self.get_index_stocks("nasdaq100").await?;
                let spx500_index_stocks = self.get_index_stocks("SPX").await?;
                let mut stock_codes = Vec::new();
                let mut add_unique_stocks = |index_stocks: Vec<Model>| {
                    for stock in index_stocks {
                        if !stock_codes.contains(&stock.code) {
                            stock_codes.push(stock.code.clone());
                            stocks.push(stock);
                        }
                    }
                };

                add_unique_stocks(nasdaq100_index_stocks);
                add_unique_stocks(spx500_index_stocks);
                Ok(stocks)
            }
        }
    }

    async fn get_upper_limit_stocks(
        &self,
        exchange: &str,
    ) -> Result<Vec<UpperLimitStock>, Box<dyn Error>> {
        // 只支持 SSE 和 SZSE
        let exchange_enum = Exchange::from_str(exchange)?;
        match exchange_enum {
            Exchange::SSE | Exchange::SZSE => {}
            _ => {
                return Err(format!("Only support SSE and SZSE, got {}", exchange).into());
            }
        }
        let application_context = APPLICATION_CONTEXT.read().await;
        let environment = application_context.get_environment().await;
        let base_url = environment
            .get_property::<String>("stock.api.akshare.baseurl").unwrap();
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        // 从 akshare 接口获取涨停池数据
        let url = format!(
            "{}/api/public/stock_zt_pool_em?date={}",
            base_url,
            date
        );
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
                .parse()?,
        );
        headers.insert("Referer", "https://emweb.securities.xinhua.com/".parse()?);
        let client = reqwest::Client::builder().build()?;
        let response = client.get(&url).headers(headers).send().await?;
        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch upper limit stocks: {}",
                response.status()
            )
            .into());
        }
        let json_data: Value = response.json().await?;

        // 解析 akshare 返回的数据
        let data_array = json_data.as_array().ok_or("Expected an array in response")?;

        let mut upper_limit_stocks = Vec::new();
        for item in data_array {
            // 提取股票代码（不含交易所后缀）
            let stock_code = item
                .get("代码")
                .and_then(|v| v.as_str())
                .ok_or("Missing stock code")?
                .to_string();

            // 拼接交易所代码后缀
            let full_code = format!("{}{}", stock_code, exchange_enum.stock_code_suffix());

            // 通过 stock_dao 检查股票是否存在于交易所
            if let Ok(Some(_stock)) = stock_dao::get_stock_by_code(&full_code).await {
                // 股票存在，添加到结果中
                let upper_limit_stock = UpperLimitStock {
                    serial_no: item
                        .get("序号")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    stock_code: full_code, // 使用拼接后的代码
                    stock_name: item
                        .get("名称")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    change_percent: item
                        .get("涨跌幅")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    latest_price: item
                        .get("最新价")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    turnover_amount: item
                        .get("成交额")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    circulating_market_cap: item
                        .get("流通市值")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    total_market_cap: item
                        .get("总市值")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    turnover_rate: item
                        .get("换手率")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    limit_up_order_fund: item
                        .get("封板资金")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    first_limit_up_time: item
                        .get("首次封板时间")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    last_limit_up_time: item
                        .get("最后封板时间")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    limit_up_break_count: item
                        .get("炸板次数")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    limit_up_stats: item
                        .get("涨停统计")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    consecutive_limit_up_days: item
                        .get("连板数")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    industry: item
                        .get("所属行业")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                };
                upper_limit_stocks.push(upper_limit_stock);
            }
        }

        Ok(upper_limit_stocks)
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

/// Earnings surprise data structure for NASDAQ API response
#[derive(Debug, Serialize, Deserialize)]
pub struct EarningsSurpriseResponse {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EarningsSurpriseTable {
    pub rows: Vec<EarningsSurpriseRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsSurpriseRow {
    #[serde(rename = "fiscalQtrEnd")]
    pub fiscal_qtr_end: String,
    #[serde(rename = "dateReported")]
    pub date_reported: String,
    pub eps: f64,
    #[serde(rename = "consensusForecast")]
    pub consensus_forecast: String,
    #[serde(rename = "percentageSurprise")]
    pub percentage_surprise: String,
}

/// Fetch earnings surprise data from NASDAQ API
pub async fn get_earnings_surprise(code: &str) -> Result<Vec<EarningsSurpriseRow>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.nasdaq.baseurl")
        .unwrap();
    let url = format!("{}/api/company/{}/earnings-surprise", base_url, code);

    info!("Fetching earnings surprise data from {}", url);

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".parse()?);

    let client = reqwest::Client::builder().build()?;
    let response = client.get(&url).headers(headers).send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch earnings surprise: {}", response.status()).into());
    }

    let earnings_response: EarningsSurpriseResponse = response.json().await?;

    // Extract rows from the JSON response
    let rows_value = earnings_response
        .data
        .get("earningsSurpriseTable")
        .and_then(|table| table.get("rows"))
        .ok_or("Missing earningsSurpriseTable.rows in response")?;

    let rows: Vec<EarningsSurpriseRow> = serde_json::from_value(rows_value.clone())?;
    Ok(rows)
}

pub async fn download(url: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".parse()?);
    headers.insert("X-Requested-With", "XMLHttpRequest".parse().unwrap());
    headers.insert(
        "Referer",
        "http://www.sse.com.cn/assortment/stock/list/share/".parse()?,
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

fn read_stocks_from_excel(
    path: &Path,
    exchange: &Exchange,
    sheet_name: &str,
    stock_code_index: usize,
    stock_name_index: usize,
) -> Result<Vec<Model>, Box<dyn Error>> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let mut stocks = Vec::new();
    let data = if ext == "xls" {
        let mut excel_xls: Xls<_> = open_workbook(path)?;
        excel_xls.worksheet_range(sheet_name)?
    } else {
        let mut excel_xlsx: Xlsx<_> = open_workbook(path)?;
        excel_xlsx.worksheet_range(sheet_name)?
    };

    for (i, row) in data.rows().enumerate() {
        if i == 0 {
            continue;
        }
        stocks.push(Model {
            code: format!("{}{}", row[stock_code_index], exchange.stock_code_suffix()),
            name: row[stock_name_index].to_string(),
            exchange: exchange.as_ref().to_string(),
            stock_type: StockKind::Stock.to_string(),
            stock_code: row[stock_code_index].to_string(),
        });
    }
    Ok(stocks)
}
