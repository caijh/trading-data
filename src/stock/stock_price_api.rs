use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use bigdecimal::num_traits::Bounded;
use chrono::{DateTime, Duration, Local, NaiveDateTime, NaiveTime, Utc};
use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::str::FromStr;
use tracing::info;
use util::request::Request;

use crate::exchange::exchange_model::Exchange;
use crate::holiday::holiday_svc::today_is_holiday;
use crate::stock::stock_model;
use crate::token::token_svc;

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
                    get_current_stock_price_from_hk(&stock.stock_code).await
                }
            }
            Exchange::NASDAQ => get_current_price_from_nasdaq(stock).await,
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
        pc: snap.get(7).unwrap().to_string(),
        p: snap.get(5).unwrap().to_string(),
        cje: snap.get(10).unwrap().to_string(),
        ud: snap.get(8).unwrap().to_string(),
        v: snap.get(9).unwrap().to_string(),
        yc: snap.get(1).unwrap().to_string(),
        t: NaiveDateTime::parse_from_str(&format!("{}{}", date, time), "%Y%m%d%H%M%S")
            .unwrap()
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

pub async fn get_stock_daily_price(
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPriceDTO>, Box<dyn Error>> {
    let exchange = Exchange::from_str(stock.exchange.as_str())?;
    info!(
        "Get stock daily price from {}, code = {}",
        exchange.as_ref(),
        stock.stock_code
    );
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let mut stock_prices = Vec::new();
    match exchange {
        Exchange::SSE => {
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
                    stock_prices.push(price);
                }
            }
            Ok(stock_prices)
        }
        Exchange::SZSE => {
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
                    stock_prices.push(price);
                }
            }
            Ok(stock_prices)
        }
        Exchange::HKEX => {
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
                "{}/hkexwidget/data/getchartdata2?hchart=1&span=6&int=5&ric={}&token={}&qid={}&callback=jQuery_{}&_={}",
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
                    let o = o.as_number().unwrap().to_string();
                    let dt: DateTime<Utc> =
                        DateTime::from_timestamp_millis(k.first().unwrap().as_i64().unwrap())
                            .unwrap();
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
                    stock_prices.push(price);
                }
                let date = Local::now().format("%Y%m%d").to_string();
                let holiday_result = today_is_holiday(exchange.as_ref()).await?;
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
                    let date = NaiveDateTime::parse_from_str(&stock_price.t, "%Y-%m-%d %H:%M:%S")
                        .unwrap()
                        .format("%Y%m%d")
                        .to_string();
                    stock_prices.push(StockDailyPriceDTO {
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
                    });
                }
            }
            Ok(stock_prices)
        }
        Exchange::NASDAQ => get_stock_daily_price_from_nasdaq(&exchange, stock).await,
    }
}

async fn get_stock_daily_price_from_nasdaq(
    exchange: &Exchange,
    stock: &stock_model::Model,
) -> Result<Vec<StockDailyPriceDTO>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;

    let url = environment
        .get_property::<String>("stock.api.nasdaq.charting")
        .unwrap();
    let now = Utc::now().with_timezone(&exchange.time_zone());
    let today = now.format("%Y-%m-%d").to_string();
    let year_day_before_now = now
        .checked_sub_signed(Duration::days(1080))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let url = format!(
        "{}/data/charting/historical?symbol={}&date={}~{}&includeLatestIntradayData=1&",
        url, &stock.stock_code, year_day_before_now, today,
    );
    info!("{:?}", url);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("Accept", "*/*".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());
    headers.insert("X-KL-Ajax-Request", "Ajax_Request".parse().unwrap());
    headers.insert(
        "Referer",
        "https://charting.nasdaq.com/dynamic/chart.html"
            .parse()
            .unwrap(),
    );
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();
    let response = client.get(&url).headers(headers).send().await?;
    let data: Value = response.json().await?;
    let kline = data.get("marketData").unwrap().as_array();
    let mut stock_prices = Vec::new();
    if let Some(kline) = kline {
        for k in kline {
            let datetime = k["Date"].as_str().unwrap();
            let datetime = NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d %H:%M:%S");
            let date = datetime.unwrap().format("%Y%m%d").to_string();
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

async fn get_current_stock_price_from_hk(code: &str) -> Result<StockPriceDTO, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.hk.baseurl")
        .unwrap();
    let token = token_svc::get_hkex_token().await;
    let timestamp = Local::now().timestamp_millis();
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
    let t = NaiveDateTime::parse_from_str(&update_time, "%Y年%m月%d日%H:%M")
        .unwrap()
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

async fn get_current_price_from_nasdaq(
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
    info!("Get stock {} daily price from url = {}", &stock.code, url);
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
    let text: Value = response.json().await?;
    let data = text.get("data").unwrap();
    let market_status = data.get("marketStatus").unwrap().as_str().unwrap();
    let primary_data = data.get("primaryData").unwrap();
    let secondary_data = data.get("secondaryData").unwrap();
    let mut price: String;
    let mut v: String;
    let mut pc: String;
    let mut ud: String;
    let update_time: String;
    if market_status == "Closed" || market_status == "Open" || market_status == "After-Hours" {
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
    } else {
        price = secondary_data["lastSalePrice"]
            .as_str()
            .unwrap()
            .to_string();
        v = secondary_data["volume"].as_str().unwrap().to_string();
        pc = secondary_data["percentageChange"]
            .as_str()
            .unwrap()
            .to_string();
        ud = primary_data["netChange"].as_str().unwrap().to_string();
        update_time = secondary_data["lastTradeTimestamp"]
            .as_str()
            .unwrap()
            .to_string();
    }
    price = price.replace("$", "").replace(",", "");
    pc = pc.replace("%", "");
    v = v.replace(",", "");
    ud = ud.replace("$", "").replace(",", "");
    let t = update_time;
    Ok(StockPriceDTO {
        h: "".to_string(),
        l: "".to_string(),
        o: "".to_string(),
        pc,
        p: price,
        cje: "".to_string(),
        ud,
        v,
        yc: "".to_string(),
        t,
    })
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

fn remove_jquery_wrapping_fn_call(data: &str) -> Value {
    // Remove the wrapping function call
    if let Some(start_idx) = data.find('(') {
        if let Some(end_idx) = data.rfind(')') {
            let json_str = &data[start_idx + 1..end_idx]; // Extract JSON string
            // Parse the JSON string
            serde_json::from_str::<Value>(json_str).unwrap()
        } else {
            serde_json::from_str::<Value>(data).unwrap()
        }
    } else {
        serde_json::from_str::<Value>(data).unwrap()
    }
}
