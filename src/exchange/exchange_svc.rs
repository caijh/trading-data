use crate::exchange::exchange_model::Exchange;
use crate::exchange::{exchange_model, market_time};
use crate::holiday::holiday_dao;
use crate::stock::stock_svc;
use application_beans::factory::bean_factory::BeanFactory;
use application_cache::CacheManager;
use application_context::context::application_context::APPLICATION_CONTEXT;
use chrono::{Datelike, Local, Utc};
use database_mysql_seaorm::Dao;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

async fn get_market_status_by_stock_code(code: &str) -> Result<String, Box<dyn Error>> {
    let stock = stock_svc::get_stock(code).await?;
    get_market_status(&stock.exchange).await
}

async fn get_market_status(exchange: &str) -> Result<String, Box<dyn Error>> {
    let exchange = exchange_model::Exchange::from_str(exchange)?;
    // 判断今天是否为周六日
    let date = Local::now().with_timezone(&exchange.time_zone());
    if date.weekday().number_from_monday() == 6 || date.weekday().number_from_monday() == 7 {
        return Ok("MarketClosed".to_string());
    }
    // 判断今天是否为节假日
    let holiday = holiday_dao::get_holiday(&exchange, &Local::now()).await?;
    if holiday.is_some() {
        return Ok("MarketClosed".to_string());
    }

    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let market_times = market_time::Entity::find()
        .filter(market_time::Column::Exchange.eq(exchange.as_ref()))
        .order_by_asc(market_time::Column::StartTime)
        .all(&dao.connection)
        .await?;
    if market_times.is_empty() {
        return Ok("MarketTrading".to_string());
    }

    let tz = exchange.time_zone();
    let time = Utc::now().with_timezone(&tz).time();
    let first = market_times.first().unwrap();
    if time < first.start_time {
        return Ok("MarketClosed".to_string());
    }
    let last = market_times.last().unwrap();
    if time > last.end_time {
        return Ok("MarketClosed".to_string());
    }
    for market_time in market_times {
        if market_time.start_time <= time && time <= market_time.end_time {
            return Ok("MarketTrading".to_string());
        }
    }

    Ok("MarketClosed".to_string())
}

pub async fn get_market_status_by_stock_code_from_cache(
    code: &str,
) -> Result<String, Box<dyn Error>> {
    let key = format!("MarketStatus:{}", code);
    let market_status = CacheManager::get_from("MarketStatus", &key).await;
    if market_status.is_some() {
        let market_status = market_status.unwrap();
        return Ok(market_status);
    }

    let market_status = get_market_status_by_stock_code(code).await?;
    CacheManager::set_to(
        "MarketStatus",
        &key,
        &market_status,
        Duration::from_secs(300),
    )
    .await;
    Ok(market_status)
}

pub async fn get_market_status_from_cache(exchange: &str) -> Result<String, Box<dyn Error>> {
    let key = format!("MarketStatus:{}", exchange);
    let market_status = CacheManager::get_from("MarketStatus", &key).await;
    if market_status.is_some() {
        let market_status = market_status.unwrap();
        return Ok(market_status);
    }

    let market_status = get_market_status(exchange).await?;
    CacheManager::set_to(
        "MarketStatus",
        &key,
        &market_status,
        Duration::from_secs(300),
    )
    .await;
    Ok(market_status)
}

/// 获取指定交易所的当前时间。
///
/// 本函数根据交易所的时区信息，获取当前的时间并格式化返回。
///
/// # 参数
/// * `exchange` - 一个字符串切片，表示交易所的名称。
///
/// # 返回值
/// * `Ok(String)` - 格式化后的当前时间字符串，格式为 "%Y-%m-%d %H:%M:%S"。
/// * `Err(Box<dyn Error>)` - 如果交易所解析失败或时区处理出现问题，则返回一个错误。
pub async fn get_current_time(exchange: &str) -> Result<String, Box<dyn Error>> {
    let exchange = Exchange::from_str(exchange)?;
    let time = Utc::now().with_timezone(&exchange.time_zone());
    Ok(time.format("%Y-%m-%d %H:%M:%S").to_string())
}
