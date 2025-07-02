use crate::exchange::exchange_model::Exchange;
use crate::stock::stock_model::Model as Stock;
use crate::stock::{stock_dao, stock_model, stock_price_dao, stock_price_model, sync_record_dao};
use application_cache::CacheManager;
use chrono::{Timelike, Utc};
use redis::Commands;
use redis_io::Redis;
use std::error::Error;
use std::str::FromStr;
use tracing::info;

pub async fn get_stock(code: &str) -> Result<stock_model::Model, Box<dyn Error>> {
    // 尝试从缓存中获取股票信息
    if let Some(cached_stock) = CacheManager::get(code).await {
        // 缓存命中，直接反序列化并返回
        return serde_json::from_str(&cached_stock)
            .map_err(|e| format!("Failed to deserialize cached stock: {}", e).into());
    }

    // 缓存未命中，从数据库中查询
    let stock = stock_dao::get_stock_by_code(code).await?;

    if stock.is_none() {
        return Err(format!("Stock {} not found or not supported", code).into());
    }
    let stock = stock.unwrap();
    // 将查询结果存入缓存
    CacheManager::set(code, &serde_json::to_string(&stock).unwrap()).await;

    Ok(stock)
}

pub async fn get_stock_daily_price(
    stock: &Stock,
) -> Result<Vec<stock_price_model::Model>, Box<dyn Error>> {
    let client = Redis::get_client();
    let mut con = client.get_connection()?;
    let key = "Stock:Price:K:D:".to_string() + &stock.code;
    let value = con.get::<&str, Option<String>>(&key)?;

    // 缓存命中，直接返回结果
    if let Some(value) = value {
        info!("Get stock daily price from cache, code = {}", stock.code);
        let prices: Vec<stock_price_model::Model> = serde_json::from_str(&value).unwrap();
        return Ok(prices);
    }

    // 缓存未命中，执行同步逻辑
    let exchange = Exchange::from_str(&stock.exchange)?;
    let date = Utc::now()
        .with_timezone(&exchange.time_zone())
        .format("%Y%m%d")
        .to_string()
        .parse::<u64>()
        .unwrap();
    let sync_record = sync_record_dao::get_sync_record(stock).await?;
    let mut updated: bool = false;
    if let Some(sync_record) = sync_record {
        updated = sync_record.date == date && sync_record.updated;
    }

    let prices = if updated {
        let prices = stock_price_dao::get_stock_prices(stock).await?;
        let now = Utc::now().with_timezone(&exchange.time_zone());
        let seconds = 3600 * 24 - now.num_seconds_from_midnight();
        con.set_ex::<&str, String, String>(
            &key,
            serde_json::to_string(&prices).unwrap(),
            seconds as u64,
        )?;
        prices
    } else {
        Vec::new()
    };

    Ok(prices)
}
