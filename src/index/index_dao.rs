use crate::exchange::exchange_model::Exchange;
use crate::index::index_model;
use crate::index::index_model::Model;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use database_mysql_seaorm::Dao;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DbErr};

pub async fn find_all() -> Result<Vec<index_model::Model>, sea_orm::DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    index_model::Entity::find().all(&dao.connection).await
}

pub async fn find_by_exchange(
    exchange: &Exchange,
) -> Result<Vec<index_model::Model>, sea_orm::DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    index_model::Entity::find()
        .filter(index_model::Column::Exchange.eq(exchange.as_ref()))
        .all(&dao.connection)
        .await
}

pub async fn get_stock_index(index: &str) -> Result<Option<Model>, DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    index_model::Entity::find_by_id(index)
        .one(&dao.connection)
        .await
}
