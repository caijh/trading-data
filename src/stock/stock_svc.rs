use crate::exchange::exchange_model::Exchange;
use crate::fund::fund_api::FundApi;
use crate::fund::{fund_dao, fund_model};
use crate::holiday::holiday_svc::today_is_holiday;
use crate::stock::stock_api::StockApi;
use crate::stock::stock_dao::get_stock_by_code;
use crate::stock::stock_model::{Model as Stock, StockKind, StockPrice};
use crate::stock::stock_price_api::{StockDailyPriceDTO, StockPriceApi};
use crate::stock::stock_price_model::Model as StockDailyPrice;
use crate::stock::{
    stock_dao, stock_model, stock_price_api, stock_price_dao, stock_price_model, sync_record_dao,
    sync_record_model,
};
use application_beans::factory::bean_factory::BeanFactory;
use application_cache::CacheManager;
use application_context::context::application_context::APPLICATION_CONTEXT;
use bigdecimal::BigDecimal;
use chrono::{Timelike, Utc};
use database_mysql_seaorm::Dao;
use redis::Commands;
use redis_io::Redis;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use std::error::Error;
use std::ops::Not;
use std::str::FromStr;
use tracing::info;

/// 异步同步指定交易所的证券和基金信息
///
/// # Arguments
/// * `exchange` - 一个字符串切片，表示要同步的交易所名称
///
/// # Returns
/// * `Result<(), Box<dyn Error>>` - 返回一个结果类型，表示操作成功或携带一个错误类型
///
/// # Remarks
/// 该函数首先会根据传入的交易所名称创建一个Exchange实例，然后同步该交易所的股票和基金信息
pub async fn sync(exchange: &str) -> Result<(), Box<dyn Error>> {
    let exchange = Exchange::from_str(exchange)?;

    // 同步股票信息
    sync_stocks(&exchange).await?;

    // 同步基金信息
    sync_funds(&exchange).await?;

    Ok(())
}

/// 同步股票信息
///
/// 该函数旨在从指定的交易所获取最新的股票信息，然后删除现有的股票信息，
/// 并保存新的股票信息。这一过程确保了股票数据的最新状态。
///
/// # 参数
///
/// * `exchange` - 一个引用，指向要从中同步股票信息的交易所。
///
/// # 返回值
///
/// 该函数返回一个 `Result` 类型，表示操作是否成功。
/// 如果操作成功，返回 `Ok(())`；如果发生错误，返回一个实现了 `Error` trait 的类型。
pub async fn sync_stocks(exchange: &Exchange) -> Result<(), Box<dyn Error>> {
    let stocks = exchange.get_stocks().await?;

    // 删除现有的股票信息，为保存最新的股票信息做准备
    delete_stocks(exchange).await?;

    // 保存从交易所获取的最新股票信息
    save_stocks(&stocks).await?;

    Ok(())
}

pub async fn sync_funds(exchange: &Exchange) -> Result<(), Box<dyn Error>> {
    let stocks = exchange.get_funds().await?;
    delete_funds(exchange).await?;
    save_stocks(&stocks).await?;
    save_funds(&stocks).await?;
    Ok(())
}

/// 保存或更新股票列表
///
/// # Arguments
///
/// * `stocks`:
///
/// returns: Result<(), Box<dyn Error, Global>>
///
/// # Examples
///
/// ```
///
/// ```
async fn save_stocks(stocks: &[Stock]) -> Result<(), Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();

    let stocks: Vec<stock_model::ActiveModel> = stocks
        .iter()
        .map(|e| e.clone().into_active_model())
        .collect();
    if stocks.is_empty().not() {
        stock_model::Entity::insert_many(stocks)
            .on_empty_do_nothing()
            .exec(&dao.connection)
            .await?;
    }
    Ok(())
}

async fn save_funds(stocks: &Vec<Stock>) -> Result<(), Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();

    let mut funds = Vec::new();
    for stock in stocks {
        funds.push(fund_model::ActiveModel {
            code: Set(stock.code.clone()),
            name: Set(stock.name.clone()),
            exchange: Set(stock.exchange.clone()),
        });
    }

    if funds.is_empty().not() {
        fund_model::Entity::insert_many(funds)
            .on_empty_do_nothing()
            .exec(&dao.connection)
            .await?;
    }

    Ok(())
}

pub async fn delete_stocks(exchange: &Exchange) -> Result<(), Box<dyn Error>> {
    stock_dao::delete_stocks_by_exchange_stock_kind(exchange, &StockKind::Stock).await?;
    Ok(())
}

pub async fn delete_funds(exchange: &Exchange) -> Result<(), Box<dyn Error>> {
    fund_dao::delete_funds_by_exchange(exchange).await?;
    stock_dao::delete_stocks_by_exchange_stock_kind(exchange, &StockKind::Fund).await?;
    Ok(())
}

pub async fn get_stock_daily_price_from_cache(
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

pub async fn get_stock_daily_price(
    code: &str,
    use_cache: bool,
) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    info!("Get stock daily price, code = {}", code);
    let stock = get_stock(code).await?;
    let mut daily_prices: Vec<StockDailyPrice> = if use_cache {
        get_stock_daily_price_from_cache(&stock).await?
    } else {
        Vec::new()
    };

    if daily_prices.is_empty() {
        let prices_dto = stock_price_api::get_stock_daily_price(&stock).await?;
        for dto in prices_dto {
            let daily_price = create_stock_daily_price(code, &dto);
            daily_prices.push(daily_price);
        }
    }

    Ok(daily_prices)
}

pub async fn sync_stock_daily_price(code: &str) -> Result<(), Box<dyn Error>> {
    let stock = get_stock(code).await?;
    let exchange = Exchange::from_str(&stock.exchange)?;
    let date = Utc::now()
        .with_timezone(&exchange.time_zone())
        .format("%Y%m%d")
        .to_string()
        .parse::<u64>()
        .unwrap();
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let sync_record = sync_record_model::Entity::find_by_id(&stock.code)
        .one(&dao.connection)
        .await?;
    // 判断是否已经同步
    let mut updated = false;
    if let Some(sync_record) = sync_record {
        updated = sync_record.date == date && sync_record.updated;
    } else {
        let record = sync_record_model::ActiveModel {
            code: Set(code.to_string()),
            date: Set(date),
            updated: Set(false),
        };
        sync_record_model::Entity::insert(record)
            .on_empty_do_nothing()
            .exec(&dao.connection)
            .await?;
    }
    info!("Sync stock {} daily price, updated = {}", code, updated);
    if !updated {
        // 从数据中获取
        let prices = stock_price_dao::get_stock_prices(&stock).await?;
        let last_price = if !prices.is_empty() {
            prices.last()
        } else {
            None
        };
        let dates: Vec<u64> = prices.iter().map(|e| e.date).collect();
        let mut new_prices = Vec::new();
        let mut price_dates = Vec::new();
        let prices_dto = stock_price_api::get_stock_daily_price(&stock).await?;
        for dto in prices_dto {
            let daily_price = create_stock_daily_price(code, &dto);
            let d = daily_price.date;
            price_dates.push(d);

            if !dates.contains(&d) {
                // 数据库中没有
                new_prices.push(daily_price.clone().into_active_model());
            }

            if stock.exchange == "HK" && last_price.is_some() && last_price.unwrap().date == d {
                // 港交所今天的数据，要到明天才更新
                let price = daily_price.clone().into_active_model();
                price.update(&dao.connection).await?;
            }
        }
        if !new_prices.is_empty() {
            stock_price_model::Entity::insert_many(new_prices)
                .exec(&dao.connection)
                .await?;
        }
        if price_dates.contains(&date) || today_is_holiday(exchange.as_ref()).await? {
            let record = sync_record_model::ActiveModel {
                code: Set(code.to_string()),
                date: Set(date),
                updated: Set(true),
            };
            sync_record_model::Entity::update(record)
                .filter(sync_record_model::Column::Code.eq(code.to_string()))
                .exec(&dao.connection)
                .await?;
        }
    }
    Ok(())
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
        amount: if dto.e.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&dto.e).unwrap())
        },
    }
}

pub async fn get_stock_price(code: &str) -> Result<StockPrice, Box<dyn Error>> {
    let stock = get_stock(code).await?;
    let exchange = Exchange::from_str(&stock.exchange)?;
    let price_dto = exchange.get_stock_price(&stock).await?;

    let price = StockPrice {
        code: code.to_string(),
        high: if price_dto.h.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.h).unwrap())
        },
        low: if price_dto.l.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.l).unwrap())
        },
        open: if price_dto.o.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.o).unwrap())
        },
        close: BigDecimal::from_str(&price_dto.p).unwrap(),
        volume: if price_dto.v.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.v).unwrap())
        },
        time: price_dto.t.clone(),
    };

    Ok(price)
}

pub async fn get_stock(code: &str) -> Result<Stock, Box<dyn Error>> {
    // 尝试从缓存中获取股票信息
    if let Some(cached_stock) = CacheManager::get(code).await {
        // 缓存命中，直接反序列化并返回
        return serde_json::from_str(&cached_stock)
            .map_err(|e| format!("Failed to deserialize cached stock: {}", e).into());
    }

    // 缓存未命中，从数据库中查询
    let stock = get_stock_by_code(code).await?;

    if stock.is_none() {
        return Err(format!("Stock {} not found or not supported", code).into());
    }
    let stock = stock.unwrap();
    // 将查询结果存入缓存
    CacheManager::set(code, &serde_json::to_string(&stock).unwrap()).await;

    Ok(stock)
}
