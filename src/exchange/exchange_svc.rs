use crate::exchange::exchange_model::Exchange;
use crate::exchange::market_time;
use crate::exchange::market_time::Model;
use crate::holiday::holiday_svc;
use crate::stock::stock_svc;
use application_beans::factory::bean_factory::BeanFactory;
use application_cache::CacheManager;
use application_context::context::application_context::APPLICATION_CONTEXT;
use chrono::{Local, NaiveTime, Utc};
use database_mysql_seaorm::Dao;
use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

/// 判断交易所当前是否处于交易状态
///
/// 逻辑流程：
/// 1. 检查今天是否为该交易所的节假日
/// 2. 获取交易所的时区并计算当前当地时间
/// 3. 检查当前时间是否落在该交易所定义的任何一个交易时间段内
async fn get_market_status(exchange: &str) -> Result<String, Box<dyn Error>> {
    // 判断今天是否为节假日
    let is_holiday = holiday_svc::is_holiday(&exchange).await?;
    if is_holiday {
        return Ok("MarketClosed".to_string());
    }

    // 根据交易所名称解析出 Exchange 模型（包含时区信息）
    let exchange = Exchange::from_str(exchange)?;
    // 获取该交易所当地的当前时间
    let date = Local::now().with_timezone(&exchange.time_zone());

    // 获取该交易所的所有交易时间段
    let market_times = get_market_times(&exchange).await?;
    if market_times.is_empty() {
        // 如果没有定义交易时间，默认视为交易中（或根据业务需求调整）
        return Ok("MarketTrading".to_string());
    }

    let time = date.time();
    let first = market_times.first().unwrap();
    if time < first.start_time {
        return Ok("MarketClosed".to_string());
    }

    let last = market_times.last().unwrap();
    if time > last.end_time {
        return Ok("MarketClosed".to_string());
    }

    // 遍历所有交易时间段，判断当前时间是否在其中
    for market_time in market_times {
        if market_time.start_time <= time && time <= market_time.end_time {
            return Ok("MarketTrading".to_string());
        }
    }

    Ok("MarketClosed".to_string())
}

/// 从缓存中获取市场状态
async fn get_market_status_cache(key: &str) -> Option<Result<String, Box<dyn Error>>> {
    let market_status = CacheManager::get_from("MarketStatus", key).await;
    if market_status.is_some() {
        let market_status = market_status.unwrap();
        return Some(Ok(market_status));
    }
    None
}

/// 获取指定股票所属市场的当前交易状态
///
/// # 参数
/// * `code` - 股票代码
///
/// # 返回值
/// * `Ok(String)` - 状态字符串 ("MarketTrading" 或 "MarketClosed")
pub async fn get_stock_market_status(code: &str) -> Result<String, Box<dyn Error>> {
    let key = format!("MarketStatus:{}", code);
    if let Some(value) = get_market_status_cache(&key).await {
        return value;
    }

    // 根据股票代码获取其所属的交易所
    let stock = stock_svc::get_stock(code).await?;
    let market_status = get_market_status(&stock.exchange).await?;

    // 缓存结果，有效期 5 分钟
    CacheManager::set_to(
        "MarketStatus",
        &key,
        &market_status,
        Duration::from_secs(300),
    )
    .await;
    Ok(market_status)
}

/// 获取指定交易所的当前交易状态
///
/// # 参数
/// * `exchange` - 交易所名称/代码
pub async fn get_exchange_market_status(exchange: &str) -> Result<String, Box<dyn Error>> {
    let key = format!("MarketStatus:{}", exchange);
    if let Some(value) = get_market_status_cache(&key).await {
        return value;
    }

    let market_status = get_market_status(exchange).await?;

    // 缓存结果，有效期 2 分钟
    CacheManager::set_to(
        "MarketStatus",
        &key,
        &market_status,
        Duration::from_secs(120),
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
pub async fn get_exchange_current_time(exchange: &str) -> Result<String, Box<dyn Error>> {
    let exchange = Exchange::from_str(exchange)?;
    let time = Utc::now().with_timezone(&exchange.time_zone());
    Ok(time.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// 从数据库查询交易所的交易时间段（内部实现）
async fn _get_market_times(exchange: &Exchange) -> Result<Vec<Model>, DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    market_time::Entity::find()
        .filter(market_time::Column::Exchange.eq(exchange.as_ref()))
        .order_by_asc(market_time::Column::StartTime)
        .all(&dao.connection)
        .await
}

/// 获取交易所的交易时间段列表
///
/// 优先从缓存中读取，若缓存失效则从数据库加载并更新缓存。
///
/// # 返回值
/// * `Ok(Vec<Model>)` - 交易时间段列表，按开始时间升序排列
pub async fn get_market_times(exchange: &Exchange) -> Result<Vec<Model>, DbErr> {
    let key = exchange.as_ref();
    let market_times_json = CacheManager::get_from("MarketTimes", key).await;
    if market_times_json.is_some() {
        let market_times_str = market_times_json.unwrap();
        let market_times: Vec<Model> =
            serde_json::from_str(&market_times_str).map_err(|e| DbErr::Custom(e.to_string()))?;
        return Ok(market_times);
    }

    let market_times = _get_market_times(exchange).await?;
    let market_times_json = serde_json::to_string(&market_times).unwrap();

    // 缓存结果，有效期 1 小时
    CacheManager::set_to(
        "MarketTimes",
        &key,
        &market_times_json,
        Duration::from_secs(10800),
    )
    .await;
    Ok(market_times)
}

/// 获取交易所最后一个交易时段的结束时间
pub async fn get_market_end_time(exchange: &Exchange) -> Result<NaiveTime, Box<dyn Error>> {
    let market_times: Vec<Model> = get_market_times(&exchange).await?;
    let last = market_times.last().unwrap();
    Ok(last.end_time)
}

/// 判断交易所是否已经收盘
pub async fn is_market_closed(exchange: &Exchange) -> Result<bool, Box<dyn Error>> {
    let date = Local::now().with_timezone(&exchange.time_zone());
    let time = date.time();
    let end_time = get_market_end_time(&exchange).await?;
    Ok(time > end_time)
}
