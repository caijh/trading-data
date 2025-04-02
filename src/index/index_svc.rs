use crate::exchange::exchange_model::Exchange;
use crate::index::index_api::IndexApi;
use crate::index::index_constituent_model::SyncIndexConstituents;
use crate::index::{index_constituent_model, index_dao, index_model};
use crate::stock::stock_svc::sync_stock_daily_price;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use database_mysql_seaorm::Dao;
use sea_orm::{ColumnTrait, ModelTrait, Set, TryIntoModel};
use sea_orm::{EntityTrait, QueryFilter};
use std::error::Error;
use std::ops::Not;
use std::str::FromStr;

/// 获取指数的成分股
///
/// # Arguments
///
/// * `index`: 指数code
///
/// returns: Result<Vec<IndexConstituent, Global>, Box<dyn Error, Global>>
///
/// # Examples
///
/// ```
///
/// ```
pub async fn get_constituent_stocks(
    index: &str,
) -> Result<Vec<index_constituent_model::Model>, Box<dyn Error>> {
    let index = get_stock_index(index).await?;
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let stocks = index_constituent_model::Entity::find()
        .filter(index_constituent_model::Column::IndexCode.eq(&index.code))
        .all(&dao.connection)
        .await?;
    Ok(stocks)
}

pub async fn sync_constituents(index: &str) -> Result<SyncIndexConstituents, Box<dyn Error>> {
    let index = get_stock_index(index).await?;
    let exchange = Exchange::from_str(&index.exchange)?;
    let stocks = exchange.get_index_stocks(&index.index_code).await?;

    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let old_constituents = index_constituent_model::Entity::find()
        .filter(index_constituent_model::Column::IndexCode.eq(&index.code))
        .all(&dao.connection)
        .await?;
    let old_constituent_codes = old_constituents
        .iter()
        .map(|c| c.stock_code.clone())
        .collect::<Vec<String>>();

    let mut constituents_to_add = Vec::new();
    let mut constituents_added = Vec::new();
    let mut stock_codes = Vec::new();
    for stock in stocks {
        if !old_constituent_codes.contains(&stock.code) {
            let constituent = index_constituent_model::ActiveModel {
                index_code: Set(index.code.to_string()),
                stock_code: Set(stock.code.to_string()),
                stock_name: Set(stock.name.to_string()),
            };
            constituents_added.push(constituent.clone().try_into_model()?);
            constituents_to_add.push(constituent);
        }
        stock_codes.push(stock.code);
    }

    let mut constituents_to_remove = Vec::new();
    for index_constituent in old_constituents.clone() {
        if !stock_codes.contains(&index_constituent.stock_code) {
            constituents_to_remove.push(index_constituent);
        }
    }

    if constituents_to_add.is_empty().not() {
        index_constituent_model::Entity::insert_many(constituents_to_add)
            .exec(&dao.connection)
            .await?;
    }
    if constituents_to_remove.is_empty().not() {
        for index_constituent in constituents_to_remove.clone() {
            index_constituent.delete(&dao.connection).await?;
        }
    }
    Ok(SyncIndexConstituents {
        old: old_constituents,
        added: constituents_added,
        removed: constituents_to_remove,
    })
}

pub async fn get_stock_index(index: &str) -> Result<index_model::Model, Box<dyn Error>> {
    let index = index_dao::get_stock_index(index).await?;
    match index {
        None => Err("Stock index is not Supported".into()),
        Some(index) => Ok(index),
    }
}

pub async fn sync_constituent_stocks_daily_price(index: &str) -> Result<(), Box<dyn Error>> {
    let stocks = get_constituent_stocks(index).await?;
    for stock in stocks {
        let _ = sync_stock_daily_price(&stock.stock_code).await;
    }
    Ok(())
}

pub async fn find_all_stock_index() -> Result<Vec<index_model::Model>, Box<dyn Error>> {
    let indexes = index_dao::find_all().await?;
    Ok(indexes)
}

pub async fn find_stock_index_by_exchange(
    exchange: &Exchange,
) -> Result<Vec<index_model::Model>, Box<dyn Error>> {
    let indexes = index_dao::find_by_exchange(exchange).await?;
    Ok(indexes)
}
