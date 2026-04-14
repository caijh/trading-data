use crate::exchange::exchange_model::Exchange;
use crate::exchange::exchange_svc;
use crate::fund::fund_api::FundApi;
use crate::fund::{fund_dao, fund_model};
use crate::index::index_job::SyncIndexStocksJob;
use crate::stock::stock_api::StockApi;
use crate::stock::stock_model::{Model as Stock, StockKind, StockPrice};
use crate::stock::stock_price_api::{StockDailyPrice, StockPriceApi};
use crate::stock::{stock_api, stock_cache, stock_dao, stock_model, stock_price_api};
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::lang::runnable::Runnable;
use bigdecimal::BigDecimal;
use chrono::Local;
use database_mysql_seaorm::Dao;
use sea_orm::ActiveValue::Set;
use sea_orm::EntityTrait;
use sea_orm::IntoActiveModel;
use std::error::Error;
use std::ops::Not;
use std::str::FromStr;
use std::u64;
use tokio::spawn;
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
    let exchange_str = exchange.to_string();
    let exchange = Exchange::from_str(exchange)?;

    // 同步股票信息
    sync_stocks(&exchange).await?;

    // 同步基金信息
    sync_funds(&exchange).await?;

    spawn(async {
        let job = SyncIndexStocksJob {
            exchange: Some(exchange_str),
        };
        job.run().await;
    });

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
    if stocks.is_empty() {
        return Ok(());
    }

    // 删除现有的股票信息，为保存最新的股票信息做准备
    delete_stocks(exchange).await?;

    // 保存从交易所获取的最新股票信息
    save_stocks(&stocks).await?;

    Ok(())
}

pub async fn sync_funds(exchange: &Exchange) -> Result<(), Box<dyn Error>> {
    let stocks = exchange.get_funds().await?;
    if stocks.is_empty() {
        return Ok(());
    }
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

pub async fn get_stock_daily_price(code: &str) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    info!("Get stock daily price, code = {}", code);
    let stock = get_stock(code).await?;
    let mut daily_prices: Vec<StockDailyPrice> =
        stock_cache::get_stock_daily_prices(&stock).await?;

    if daily_prices.is_empty() {
        let prices = stock_price_api::get_stock_daily_price(&stock).await?;
        daily_prices = prices;
        let exchange = Exchange::from_str(stock.exchange.as_str())?;
        let market_closed = exchange_svc::is_market_closed(&exchange).await?;
        if market_closed {
            // Fix akshare's daily price data, which missing the latest day
            // (exchange == SSE or exchange == SZSE) and stock_type == StockKind::Stock的情况下，判断daily_prices中是否包含最新的交易日，如果不包含，则从股票价格接口获取最新的价格，并添加到daily_prices中
            if (exchange.as_ref() == Exchange::SSE.as_ref()
                || exchange.as_ref() == Exchange::SZSE.as_ref())
                && stock.stock_type == StockKind::Stock.to_string()
            {
                if let Some(last_price) = daily_prices.last() {
                    let last_price_date = &last_price.time;
                    let date = Local::now().with_timezone(&exchange.time_zone());
                    let date = date.format("%Y%m%d").to_string().parse::<u64>()?;
                    if date > *last_price_date {
                        let latest_price = get_latest_price(&stock).await?;
                        daily_prices.push(StockDailyPrice {
                            open: latest_price.open.unwrap(),
                            close: latest_price.close,
                            low: latest_price.low.unwrap(),
                            high: latest_price.high.unwrap(),
                            volume: latest_price.volume,
                            time: date,
                        });
                    }
                }
            }
            stock_cache::set_stock_daily_prices(&stock, &daily_prices).await?;
        }
    }

    Ok(daily_prices)
}

pub async fn get_stock_prices(code: &str) -> Result<Vec<StockDailyPrice>, Box<dyn Error>> {
    let prices = get_stock_daily_price(code).await?;
    Ok(prices)
}

pub async fn get_stock_price(code: &str) -> Result<StockPrice, Box<dyn Error>> {
    let stock = get_stock(code).await?;
    let price = get_latest_price(&stock).await?;
    Ok(price)
}

pub async fn get_latest_price(stock: &Stock) -> Result<StockPrice, Box<dyn Error>> {
    let exchange = Exchange::from_str(&stock.exchange)?;
    let price_dto = exchange.get_stock_price(&stock).await?;
    let price = StockPrice {
        code: stock.code.to_string(),
        high: if price_dto.h.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.h)?)
        },
        low: if price_dto.l.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.l)?)
        },
        open: if price_dto.o.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.o)?)
        },
        close: BigDecimal::from_str(&price_dto.p)?,
        volume: if price_dto.v.is_empty() {
            None
        } else {
            Some(BigDecimal::from_str(&price_dto.v)?)
        },
        time: price_dto.t.clone(),
    };

    Ok(price)
}

pub async fn get_stock(code: &str) -> Result<Stock, Box<dyn Error>> {
    stock_cache::get_stock(code).await
}

/// Get earnings surprise data for NASDAQ stocks
pub async fn get_earnings_surprise(
    code: &str,
) -> Result<Vec<stock_api::EarningsSurpriseRow>, Box<dyn Error>> {
    let stock = get_stock(code).await?;

    // Only NASDAQ stocks have earnings surprise data
    if stock.exchange != Exchange::NASDAQ.as_ref() {
        return Err(format!(
            "Earnings surprise data is only available for NASDAQ stocks, got {}",
            stock.exchange
        )
        .into());
    }

    // Use the stock_code (without suffix) for the API call
    let earnings_data = stock_api::get_earnings_surprise(&stock.stock_code).await?;
    Ok(earnings_data)
}
