use application_cache::CacheManager;
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use bigdecimal::num_traits::Bounded;
use chrono::{DateTime, Duration, Local, NaiveDateTime, NaiveTime, Utc};
use rand::{RngExt, rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::str::FromStr;
use tracing::info;
use util::request::Request;
use crate::exchange::exchange_model::Exchange;
use crate::holiday::holiday_svc::is_holiday;
use crate::stock::stock_model;
use crate::token::token_svc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockDailyPriceDTO {
    pub d: String,
    pub o: String,
    pub h: String,
    pub l: String,
    pub c: String,
    pub v: String,
    pub e: String,
    pub zd: String,
    pub zdf: String,
    pub hs: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockDailyPrice {
    /// 股票代码
    pub code: String,
    /// 交易日期
    pub date: u64,
    /// 当日开盘价
    pub open: BigDecimal,
    /// 当日收盘价
    pub close: BigDecimal,
    /// 当日最高价
    pub high: BigDecimal,
    /// 当日最低价
    pub low: BigDecimal,
    /// 当日成交量，可能为空
    pub volume: Option<BigDecimal>,
}

fn create_stock_daily_price(code: &str, dto: &StockDailyPriceDTO) -> StockDailyPrice {
    StockDailyPrice {
        code: code.to_string(),
        date: dto.d.parse::<u64>().unwrap(),
        open: BigDecimal::from_str(&dto.o).unwrap(),
        close: BigDecimal::from_str(&dto.c).unwrap(),
        high: BigDecimal::from_str(&dto.h).unwrap(),
        low: BigDecimal::from_str(&dto.l).unwrap(),
        volume: Some(BigDecimal::from_str(&dto.v).unwrap()),
    }
}

/// Helper function to convert ISO date string to u64 date format
fn iso_date_to_u64(date_str: &str) -> Result<u64, Box<dyn Error>> {
    let datetime = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S.%f")?;
    Ok(datetime.format("%Y%m%d").to_string().parse::<u64>()?)
}

/// Helper function to get akshare base URL from environment
async fn get_akshare_base_url() -> Result<String, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    environment
        .get_property::<String>("stock.api.akshare.baseurl")
        .ok_or_else(|| "Missing property: stock.api.akshare.baseurl".into())
}

/// Helper function to transform stock code to akshare format
fn to_akshare_symbol(exchange: &Exchange, stock_code: &str) -> String {
    match exchange {
        Exchange::SSE => format!("sh{}", stock_code),
        Exchange::SZSE => format!("sz{}", stock_code),
        _ => stock_code.to_string(),
    }
}

/// Helper function to parse akshare kline data and convert to StockDailyPrice
async fn parse_akshare_kline(
    url: &str,
    stock_code: &str,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    info!("Get stock daily price from akshare: {}", url);
    let response = Request::get_response(url).await?;
    let data: Value = response.json().await?;
    let kline = data.as_array();
    let mut stock_prices = Vec::new();
    if let Some(kline) = kline {
        stock_prices.reserve(kline.len());
        for k in kline {
            let date_str = k["date"].as_str().unwrap();
            let date = iso_date_to_u64(date_str)?;
            let price = StockDailyPriceDTO {
                d: date.to_string(),
                o: k["open"].as_f64().unwrap().to_string(),
                c: k["close"].as_f64().unwrap().to_string(),
                l: k["low"].as_f64().unwrap().to_string(),
                h: k["high"].as_f64().unwrap().to_string(),
                zd: String::new(),
                zdf: String::new(),
                v: k["volume"].as_number().unwrap().to_string(),
                e: String::new(),
                hs: String::new(),
            };
            let price = create_stock_daily_price(stock_code, &price);
            stock_prices.push(price);
        }
    }
    Ok(stock_prices)
}

#[async_trait]
pub trait StockPriceApi {
    async fn get_stock_price(
        &self,
        stock: &stock_model::Model,
    ) -> Result<StockPriceDTO, Box<dyn Error>>;
}

#[async_trait]
impl StockPriceApi for Exchange {
    async fn get_stock_price(
        &self,
        stock: &stock_model::Model,
    ) -> Result<StockPriceDTO, Box<dyn Error>> {
        match self {
            Exchange::SSE => get_current_price_from_sse(&stock.stock_code).await,
            Exchange::SZSE => get_current_price_from_szse(&stock.stock_code).await,
            Exchange::HKEX => {
                if stock.stock_type == "Index" {
                    get_current_index_price_from_hk(self, &stock.stock_code).await
                } else {
                    get_current_stock_price_from_hk(self, &stock.stock_code).await
                }
            }
            Exchange::NASDAQ => get_current_price_from_nasdaq(self, stock).await,
        }
    }
}

async fn get_current_price_from_sse(code: &str) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.sh.baseurl")
        .unwrap();
    let url = format!(
        "{}/v1/sh1/snap/{}?_={}",
        base_url,
        code,
        Local::now().timestamp_millis()
    );
    info!("Get stock {} daily price from url = {}", code, url);
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let json: Value = response.json().await?;
    let snap = json.get("snap").unwrap();
    let date = json.get("date").unwrap().to_string();
    let time = json.get("time").unwrap().to_string();
    let time = if time.len() < 6 {
        format!("{}{}", 0, time)
    } else {
        time
    };
    Ok(StockPriceDTO {
        h: snap.get(3).unwrap().to_string(),
        l: snap.get(4).unwrap().to_string(),
        o: snap.get(2).unwrap().to_string(),
        pc: snap.get(6).unwrap().to_string(),
        p: snap.get(5).unwrap().to_string(),
        cje: snap.get(9).unwrap().to_string(),
        ud: snap.get(7).unwrap().to_string(),
        v: snap.get(8).unwrap().to_string(),
        yc: snap.get(1).unwrap().to_string(),
        t: NaiveDateTime::parse_from_str(&format!("{}{}", date, time), "%Y%m%d%H%M%S")?
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    })
}

async fn get_current_price_from_szse(code: &str) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.sz.baseurl")
        .unwrap();
    let url = format!(
        "{}/api/market/ssjjhq/getTimeData?random={}&marketId=1&code={}",
        base_url,
        rng().random::<f64>(),
        code
    );
    info!("Get stock {} daily price from url = {}", code, url);
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let json: Value = response.json().await?;
    let data = json.get("data").unwrap();
    Ok(StockPriceDTO {
        h: data["high"].as_str().unwrap().to_string(),
        l: data["low"].as_str().unwrap().to_string(),
        o: data["open"].as_str().unwrap().to_string(),
        pc: data["deltaPercent"].as_str().unwrap().to_string(),
        p: data["now"].as_str().unwrap().to_string(),
        cje: data["amount"].as_number().unwrap().to_string(),
        ud: data["delta"].as_str().unwrap().to_string(),
        v: data["volume"].as_number().unwrap().to_string(),
        yc: "".to_string(),
        t: data["marketTime"].as_str().unwrap().to_string(),
    })
}

async fn get_stock_daily_price_from_sse(
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let mut stock_prices = Vec::new();
    let url = environment
        .get_property::<String>("stock.api.sh.baseurl")
        .unwrap();
    let url = format!(
        "{}/v1/sh1/dayk/{}?begin=-1000&end=-1&period=day&_={}",
        url,
        &stock.stock_code,
        Local::now().timestamp_millis()
    );
    let response = Request::get_response(&url).await?;
    let json: Value = response.json().await?;
    let kline = json.get("kline").unwrap().as_array();
    if let Some(kline) = kline {
        for k in kline {
            let k = k.as_array().unwrap();
            let price = StockDailyPriceDTO {
                d: k.first().unwrap().to_string(),
                o: k.get(1).unwrap().to_string(),
                h: k.get(2).unwrap().to_string(),
                l: k.get(3).unwrap().to_string(),
                c: k.get(4).unwrap().to_string(),
                v: k.get(5).unwrap().to_string(),
                e: k.get(6).unwrap().to_string(),
                zd: "".to_string(),
                zdf: "".to_string(),
                hs: "".to_string(),
            };
            let price = create_stock_daily_price(&stock.code, &price);
            stock_prices.push(price);
        }
    }
    Ok(stock_prices)
}

async fn get_stock_daily_price_from_szse(
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let mut stock_prices = Vec::new();
    let url = environment
        .get_property::<String>("stock.api.sz.baseurl")
        .unwrap();
    let url = format!(
        "{}/api/market/ssjjhq/getHistoryData?random={}&cycleType=32&marketId=1&code={}",
        url,
        rng().random::<f64>(),
        &stock.stock_code
    );
    let response = Request::get_response(&url).await?;
    let json: Value = response.json().await?;
    let kline = json
        .get("data")
        .unwrap()
        .get("picupdata")
        .unwrap()
        .as_array();
    if let Some(kline) = kline {
        for k in kline {
            let k = k.as_array().unwrap();
            let price = StockDailyPriceDTO {
                d: k.first()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
                    .replace('-', ""),
                o: k.get(1).unwrap().as_str().unwrap().to_string(),
                c: k.get(2).unwrap().as_str().unwrap().to_string(),
                l: k.get(3).unwrap().as_str().unwrap().to_string(),
                h: k.get(4).unwrap().as_str().unwrap().to_string(),
                zd: k.get(5).unwrap().as_str().unwrap().to_string(),
                zdf: k.get(6).unwrap().as_str().unwrap().to_string(),
                v: k.get(7).unwrap().to_string(),
                e: k.get(8).unwrap().to_string(),
                hs: "".to_string(),
            };
            let price = create_stock_daily_price(&stock.code, &price);
            stock_prices.push(price);
        }
    }
    Ok(stock_prices)
}

async fn get_stock_daily_price_from_hkex(
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let exchange = Exchange::from_str(stock.exchange.as_str())?;
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let mut stock_prices = Vec::new();
    let url = environment
        .get_property::<String>("stock.api.hk.baseurl")
        .unwrap();
    let token = token_svc::get_hkex_token().await;
    let timestramp = Local::now().timestamp_millis();
    let code = if stock.stock_type == "Index" {
        format!(".{}", stock.stock_code)
    } else {
        format!("{:0>4}.HK", stock.stock_code)
    };
    let url = format!(
        "{}/hkexwidget/data/getchartdata2?hchart=1&span=6&int=7&ric={}&token={}&qid={}&callback=jQuery_{}&_={}",
        url, code, token, timestramp, timestramp, timestramp,
    );
    let response = Request::get_response(&url).await?;
    let text = response.text().await?;
    let json = remove_jquery_wrapping_fn_call(&text);
    let kline = json
        .get("data")
        .unwrap()
        .get("datalist")
        .unwrap()
        .as_array();
    if let Some(kline) = kline {
        let mut dates = Vec::new();
        for k in kline {
            let k = k.as_array().unwrap();
            let o = k.get(1).unwrap();
            if o.is_null() {
                continue;
            }
            if o.as_f64().unwrap() < 0.0 {
                continue;
            }
            let o = o.as_number().unwrap().to_string();
            let dt: DateTime<Utc> =
                DateTime::from_timestamp_millis(k.first().unwrap().as_i64().unwrap()).unwrap();
            let date = dt.with_timezone(&Local).format("%Y%m%d").to_string();
            dates.push(date.clone());
            let price = StockDailyPriceDTO {
                d: date,
                o,
                c: k.get(4).unwrap().as_number().unwrap().to_string(),
                l: k.get(3).unwrap().as_number().unwrap().to_string(),
                h: k.get(2).unwrap().as_number().unwrap().to_string(),
                zd: "".to_string(),
                zdf: "".to_string(),
                v: k.get(5).unwrap().as_number().unwrap().to_string(),
                e: k.get(6).unwrap().as_number().unwrap().to_string(),
                hs: "".to_string(),
            };
            let price = create_stock_daily_price(&stock.code, &price);
            stock_prices.push(price);
        }
        let date = Local::now().format("%Y%m%d").to_string();
        let holiday_result = is_holiday(exchange.as_ref()).await?;
        // new time from Local::now with 9:30
        let open_time = Local::now()
            .with_time(NaiveTime::from_hms_opt(9, 30, 0).unwrap())
            .unwrap();
        if stock.stock_type == "Stock"
            && Local::now() > open_time
            && !dates.contains(&date)
            && !holiday_result
        {
            // append today price
            let stock_price = exchange.get_stock_price(&stock).await?;
            let date = NaiveDateTime::parse_from_str(&stock_price.t, "%Y-%m-%d %H:%M:%S")?
                .format("%Y%m%d")
                .to_string();
            let dto = StockDailyPriceDTO {
                d: date,
                o: stock_price.o,
                h: stock_price.h,
                l: stock_price.l,
                c: stock_price.p,
                v: stock_price.v,
                e: stock_price.cje,
                zd: stock_price.ud,
                zdf: stock_price.pc,
                hs: "".to_string(),
            };
            let price = create_stock_daily_price(&stock.code, &dto);
            stock_prices.push(price);
        }
    }
    Ok(stock_prices)
}

async fn get_stock_daily_price_from_akshare_zh_a(
    exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let base_url = get_akshare_base_url().await?;
    let symbol = to_akshare_symbol(exchange, &stock.stock_code);
    let url = format!(
        "{}/api/public/stock_zh_a_daily?symbol={}&adjust=qfq",
        base_url, symbol
    );
    parse_akshare_kline(&url, &stock.code).await
}

pub async fn get_stock_daily_price(
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let exchange = Exchange::from_str(stock.exchange.as_str())?;
    info!(
        "Get stock daily price from {}, code = {}, type = {}",
        exchange.as_ref(),
        stock.stock_code,
        stock.stock_type
    );
    match exchange {
        Exchange::SSE => {
            if stock.stock_type == "Stock" {
                get_stock_daily_price_from_akshare_zh_a(&exchange, stock).await
            } else {
                get_stock_daily_price_from_sse(stock).await
            }
        }
        Exchange::SZSE => {
            if stock.stock_type == "Stock" {
                get_stock_daily_price_from_akshare_zh_a(&exchange, stock).await
            } else {
                get_stock_daily_price_from_szse(stock).await
            }
        }
        Exchange::HKEX => get_stock_daily_price_from_hkex(stock).await,
        Exchange::NASDAQ => {
            let code = &stock.code;
            if code == "NDX.NS" || code == "SPX.NS" || code == "IXIC.NS" {
                let symbol = if code == "SPX.NS" {
                    ".INX"
                } else if code == "NDX.NS" {
                    ".NDX"
                } else {
                    ".IXIC"
                };
                get_index_stock_daily_price_from_akshare(&exchange, stock, symbol).await
            } else if regex::Regex::new(r"^[A-Z]+\.[A-Z]+\.NS$")?.is_match(code) {
                get_stock_daily_price_from_akshare(&exchange, stock).await
            } else {
                get_stock_daily_price_from_nasdaq(&exchange, stock).await
            }
        }
    }
}

async fn get_index_stock_daily_price_from_akshare(
    _exchange: &Exchange,
    stock: &stock_model::Model,
    symbol: &str,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let base_url = get_akshare_base_url().await?;
    let url = format!(
        "{}/api/public/index_us_stock_sina?symbol={}",
        base_url, symbol
    );
    parse_akshare_kline(&url, &stock.code).await
}

async fn get_stock_daily_price_from_nasdaq(
    exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let url = environment
        .get_property::<String>("stock.api.nasdaq.charting")
        .unwrap();
    let now = Utc::now().with_timezone(&exchange.time_zone());
    let today = now.format("%Y-%m-%d").to_string();
    let year_day_before_now = now
        .checked_sub_signed(Duration::days(2000))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let url = format!(
        "{}/data/charting/historical?symbol={}&date={}~{}&includeLatestIntradayData=1&",
        url, &stock.stock_code, year_day_before_now, today,
    );
    info!("{:?}", url);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            .parse()?,
    );
    headers.insert("Accept", "*/*".parse()?);
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);
    headers.insert("X-KL-Ajax-Request", "Ajax_Request".parse()?);
    headers.insert(
        "Referer",
        "https://charting.nasdaq.com/dynamic/chart.html".parse()?,
    );
    let client = reqwest::Client::builder().cookie_store(true).build()?;
    let response = client.get(&url).headers(headers).send().await?;
    let data: Value = response.json().await?;
    let kline = data.get("marketData").unwrap().as_array();
    let mut stock_prices = Vec::new();
    if let Some(kline) = kline {
        for k in kline {
            let datetime = k["Date"].as_str().unwrap();
            let datetime = NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M:%S");
            let date = datetime?.format("%Y%m%d").to_string();
            let price = StockDailyPriceDTO {
                d: date,
                o: k["Open"].as_number().unwrap().to_string(),
                c: k["Close"].as_number().unwrap().to_string(),
                l: k["Low"].as_number().unwrap().to_string(),
                h: k["High"].as_number().unwrap().to_string(),
                zd: "".to_string(),
                zdf: "".to_string(),
                v: k["Volume"].as_number().unwrap().to_string(),
                e: "".to_string(),
                hs: "".to_string(),
            };
            let price = create_stock_daily_price(&stock.code, &price);
            stock_prices.push(price);
        }
    }
    Ok(stock_prices)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockPriceDTO {
    // 最高价
    pub h: String,
    // 最低价
    pub l: String,
    // 开盘价
    pub o: String,
    // 涨跌幅（%）
    pub pc: String,
    // 当前价
    pub p: String,
    // 成交额（元）
    pub cje: String,
    // 涨跌额（元）
    pub ud: String,
    // 成交量（手）
    pub v: String,
    // 昨收
    pub yc: String,
    // 时间
    pub t: String,
}

async fn get_current_stock_price_from_hk(
    exchange: &Exchange,
    code: &str,
) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.hk.baseurl")
        .unwrap();
    let token = token_svc::get_hkex_token().await;
    let timestamp = Utc::now()
        .with_timezone(&exchange.time_zone())
        .timestamp_millis();
    let url = format!(
        "{}/hkexwidget/data/getequityquote?sym={}&token={}&lang=chi&qid={}&callback=jQuery_{}&_={}",
        base_url, code, token, timestamp, timestamp, timestamp,
    );
    info!("Get stock {} daily price from url = {}", code, url);
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    let json = remove_jquery_wrapping_fn_call(&text);
    let data = json.get("data").unwrap();
    let data = data.get("quote").unwrap();
    let v = data["vo"].as_str().unwrap().to_string();
    let vo_u = data["vo_u"].as_str().unwrap().to_string();
    let v = cal_value(&v, &vo_u);
    let am = data["am"].as_str().unwrap().to_string();
    let am_u = data["am_u"].as_str().unwrap().to_string();
    let am = cal_value(&am, &am_u);
    let update_time = data["updatetime"].as_str().unwrap().to_string();
    let t = NaiveDateTime::parse_from_str(&update_time, "%Y年%m月%d日%H:%M")?
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    Ok(StockPriceDTO {
        h: data["hi"].as_str().unwrap().to_string(),
        l: data["lo"].as_str().unwrap().to_string(),
        o: data["op"].as_str().unwrap().to_string(),
        pc: data["pc"].as_str().unwrap().to_string(),
        p: data["ls"].as_str().unwrap().to_string(),
        cje: am.to_string(),
        ud: data["nc"].as_str().unwrap().to_string(),
        v: v.to_string(),
        yc: data["hc"].as_str().unwrap().to_string(),
        t,
    })
}

async fn get_current_index_price_from_hk(
    exchange: &Exchange,
    code: &str,
) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.hk.baseurl")
        .unwrap();
    let token = token_svc::get_hkex_token().await;
    let timestamp = Utc::now()
        .with_timezone(&exchange.time_zone())
        .timestamp_millis();
    let url = format!(
        "{}/hkexwidget/data/getchartdata2?hchart=1&span=0&int=0&ric=.{}&token={}&qid={}&callback=jQuery_{}&_={}",
        base_url, code, token, timestamp, timestamp, timestamp,
    );
    info!("Get stock {} daily price from url = {}", code, url);
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    let json = remove_jquery_wrapping_fn_call(&text);
    let data = json.get("data").unwrap();
    let datalist = data.get("datalist").unwrap().as_array();
    let mut open = 0f64;
    let mut high = 0f64;
    let mut low = f64::max_value();
    let mut volume = 0f64;
    let mut amount = 0f64;
    let mut t = "".to_string();
    let mut price = 0f64;
    if let Some(klines) = datalist {
        for k in klines {
            let k = k.as_array().unwrap();
            let o = k.get(1).unwrap();
            if o.is_null() {
                continue;
            }
            let dt: DateTime<Utc> =
                DateTime::from_timestamp_millis(k.first().unwrap().as_i64().unwrap()).unwrap();
            t = dt
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            let o = o.as_f64().unwrap();
            if open == 0f64 {
                open = o;
            }
            let h = k.get(2).unwrap().as_f64().unwrap();
            if h > high {
                high = h;
            }
            let l = k.get(3).unwrap().as_f64().unwrap();
            if l < low {
                low = l;
            }
            let v = k.get(5).unwrap().as_f64().unwrap();
            volume += v;
            let e = k.get(6).unwrap().as_f64().unwrap();
            amount += e;
            let c = k.get(4).unwrap().as_f64().unwrap();
            price = c;
        }
    }
    Ok(StockPriceDTO {
        h: high.to_string(),
        l: low.to_string(),
        o: open.to_string(),
        pc: "".to_string(),
        p: price.to_string(),
        cje: amount.to_string(),
        ud: "".to_string(),
        v: volume.to_string(),
        yc: "".to_string(),
        t,
    })
}

async fn get_latest_intraday_data_from_nasdaq(
    exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let url = environment
        .get_property::<String>("stock.api.nasdaq.charting")
        .unwrap();
    let now = Utc::now().with_timezone(&exchange.time_zone());
    let today = now.format("%Y-%m-%d").to_string();
    let url = format!(
        "{}/data/charting/historical?symbol={}&date={}~&includeLatestIntradayData=1&",
        url, &stock.stock_code, today,
    );
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            .parse()?,
    );
    headers.insert("Accept", "*/*".parse()?);
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);
    headers.insert("X-KL-Ajax-Request", "Ajax_Request".parse()?);
    headers.insert(
        "Referer",
        "https://charting.nasdaq.com/dynamic/chart.html".parse()?,
    );
    let client = reqwest::Client::builder().cookie_store(true).build()?;
    let response = client.get(&url).headers(headers).send().await?;
    let data: Value = response.json().await?;
    let latest_intraday_data = data.get("latestIntradayData").unwrap();

    // Round to 3 decimal places before converting to string
    let open_value = latest_intraday_data
        .get("Open")
        .unwrap()
        .as_f64()
        .unwrap();
    let open = (open_value * 1000.0).round() / 1000.0;
    let open = open.to_string();

    let close_value = latest_intraday_data
        .get("Close")
        .unwrap()
        .as_f64()
        .unwrap();
    let close = (close_value * 1000.0).round() / 1000.0;
    let close = close.to_string();

    let low_value = latest_intraday_data
        .get("Low")
        .unwrap()
        .as_f64()
        .unwrap();
    let low = (low_value * 1000.0).round() / 1000.0;
    let low = low.to_string();

    let high_value = latest_intraday_data
        .get("High")
        .unwrap()
        .as_f64()
        .unwrap();
    let high = (high_value * 1000.0).round() / 1000.0;
    let high = high.to_string();

    let volume = latest_intraday_data
        .get("Volume")
        .unwrap()
        .as_f64()
        .unwrap()
        .to_string();
    let ud = latest_intraday_data
        .get("Change")
        .unwrap()
        .as_f64()
        .unwrap()
        .to_string();
    let date = latest_intraday_data
        .get("Date")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    Ok(StockPriceDTO {
        h: high,
        l: low,
        o: open,
        pc: "".to_string(),
        p: close,
        cje: "".to_string(),
        ud: ud,
        v: volume,
        yc: "".to_string(),
        t: date,
    })
}

async fn get_open_price_from_nasdaq(
    exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<String, Box<dyn Error>> {
    let cached_open_price = CacheManager::get_from("OpenPrice", &stock.code).await;
    if let Some(open_price) = cached_open_price {
        return Ok(open_price);
    }
    let stock_price = get_latest_intraday_data_from_nasdaq(exchange, stock).await?;
    let open = stock_price.o;
    if open != "0" && !open.is_empty() {
        CacheManager::set_to(
        "OpenPrice",
        &stock.code,
        &open,
        core::time::Duration::from_hours(6),
        )
        .await;
    }
    Ok(open)
}

async fn get_current_price_from_nasdaq(
    exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.nasdaq.baseurl")
        .unwrap();
    let url = if stock.stock_type == "Index" {
        format!(
            "{}/api/quote/{}/info?assetclass=index",
            base_url, &stock.stock_code
        )
    } else if stock.stock_type == "Fund" {
        format!(
            "{}/api/quote/{}/info?assetclass=etf",
            base_url, &stock.stock_code
        )
    } else {
        format!(
            "{}/api/quote/{}/info?assetclass=stocks",
            base_url, &stock.stock_code
        )
    };
    info!("Get stock {} price from url = {}", &stock.code, url);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            .parse()?,
    );
    headers.insert("Accept", "*/*".parse()?);
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);
    let client = reqwest::Client::builder().cookie_store(true).build()?;
    let response = client.get(&url).headers(headers).send().await?;
    let text: Value = response.json().await?;
    let data = text.get("data").unwrap();
    let market_status = data.get("marketStatus").unwrap().as_str().unwrap();
    match market_status {
        "Closed" | "After-Hours" | "Pre-Market" => {
            get_latest_intraday_data_from_nasdaq(exchange, stock).await
        }
        _ => {
            let primary_data = data.get("primaryData").unwrap();
            let key_stats = data.get("keyStats").unwrap();
            let mut price: String;
            let mut v: String;
            let mut pc: String;
            let mut ud: String;
            let update_time: String;
            price = primary_data["lastSalePrice"].as_str().unwrap().to_string();
            v = primary_data["volume"].as_str().unwrap().to_string();
            pc = primary_data["percentageChange"]
                .as_str()
                .unwrap()
                .to_string();
            ud = primary_data["netChange"].as_str().unwrap().to_string();
            update_time = primary_data["lastTradeTimestamp"]
                .as_str()
                .unwrap()
                .to_string();
            price = price.replace("$", "").replace(",", "");
            pc = pc.replace("%", "");
            v = v.replace(",", "");
            ud = ud.replace("$", "").replace(",", "");
            // Parse high and low from keyStats.dayrange.value (format: "470.00 - 476.75")
            let (high, low) = parse_dayrange(key_stats);
            // 单独请求历史接口获取开盘价
            let open = get_open_price_from_nasdaq(exchange, &stock)
                .await
                .unwrap_or_default();
            let t = NaiveDateTime::parse_from_str(&update_time[..update_time.len()-3], "%b %d, %Y %I:%M %p")?
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            Ok(StockPriceDTO {
                h: high,
                l: low,
                o: open,
                pc,
                p: price,
                cje: "".to_string(),
                ud,
                v,
                yc: "".to_string(),
                t,
            })
        }
    }
}

/// Parse dayrange value from keyStats to extract high and low prices
/// Format: "470.00 - 476.75" -> (high: "476.75", low: "470.00")
fn parse_dayrange(key_stats: &Value) -> (String, String) {
    let dayrange = key_stats
        .get("dayrange")
        .and_then(|d| d.get("value"))
        .and_then(|v| v.as_str());
    if let Some(dayrange_str) = dayrange {
        let parts: Vec<&str> = dayrange_str.splitn(2, " - ").collect();
        if parts.len() == 2 {
            let low = parts[0].trim().to_string();
            let high = parts[1].trim().to_string();
            return (high, low);
        }
    }
    (String::new(), String::new())
}

fn cal_value(val: &str, unit: &str) -> BigDecimal {
    if val.is_empty() {
        return BigDecimal::from(0);
    }
    let val = BigDecimal::from_str(val).unwrap();
    let unit = match unit {
        "B" => BigDecimal::from(1000000000),
        "M" => BigDecimal::from(1000000),
        "K" => BigDecimal::from(1000),
        _ => BigDecimal::from(1),
    };
    val * unit
}

pub fn remove_jquery_wrapping_fn_call(data: &str) -> Value {
    // Remove the wrapping function call
    if let (Some(start_idx), Some(end_idx)) = (data.find('('), data.rfind(')')) {
        if end_idx > start_idx {
            let json_str = &data[start_idx + 1..end_idx];
            serde_json::from_str::<Value>(json_str)
                .unwrap_or_else(|_| serde_json::from_str::<Value>(data).unwrap())
        } else {
            serde_json::from_str::<Value>(data).unwrap()
        }
    } else {
        serde_json::from_str::<Value>(data).unwrap()
    }
}

async fn get_stock_daily_price_from_akshare(
    _exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let base_url = get_akshare_base_url().await?;
    let url = format!(
        "{}/api/public/stock_us_daily?symbol={}&adjust=qfq",
        base_url, stock.stock_code
    );
    parse_akshare_kline(&url, &stock.code).await
}
