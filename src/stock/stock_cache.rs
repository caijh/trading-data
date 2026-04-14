use crate::stock::stock_model::Model as Stock;
use crate::stock::stock_price_api::StockDailyPrice;
use crate::stock::{stock_dao, stock_model};
use application_cache::CacheManager;
use redis::Commands;
use redis_io::Redis;
use std::error::Error;
use std::time::Duration;
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
    CacheManager::set_to(
        "",
        code,
        &serde_json::to_string(&stock)?,
        Duration::from_secs(3600 * 3),
    )
    .await;

    Ok(stock)
}

pub async fn get_stock_daily_prices(stock: &Stock) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let client = Redis::get_client();
    let mut con = client.get_connection()?;
    let key = "Stock:Price:K:D:".to_string() + &stock.code;
    let value = con.get::<&str, Option<String>>(&key)?;

    // 缓存命中，直接返回结果
    if let Some(value) = value {
        info!("Get stock daily price from cache, code = {}", stock.code);
        let prices: Vec<StockDailyPrice> = serde_json::from_str(&value)?;
        return Ok(prices);
    }

    Ok(Vec::new())
}

pub async fn set_stock_daily_prices(
    stock: &Stock,
    prices: &Vec<StockDailyPrice>,
) -> Result<(), Box<dyn Error>> {
    let client = Redis::get_client();
    let mut con = client.get_connection()?;
    let key = "Stock:Price:K:D:".to_string() + &stock.code;
    let seconds = 60 * 5;
    con.set_ex::<&str, String, String>(
        &key,
        serde_json::to_string(&prices).unwrap(),
        seconds as u64,
    )?;
    Ok(())
}
