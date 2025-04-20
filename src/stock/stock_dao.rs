use crate::exchange::exchange_model::Exchange;
use crate::stock::stock_model;
use crate::stock::stock_model::StockKind;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use database_mysql_seaorm::Dao;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DbErr, DeleteResult};

pub async fn delete_stocks_by_exchange_stock_kind(
    exchange: &Exchange,
    stock_kind: &StockKind,
) -> Result<DeleteResult, DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();

    stock_model::Entity::delete_many()
        .filter(stock_model::Column::Exchange.eq(exchange.as_ref()))
        .filter(stock_model::Column::StockType.eq(stock_kind.to_string()))
        .exec(&dao.connection)
        .await
}
