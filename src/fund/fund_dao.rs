use crate::fund::fund_model;
use crate::fund::fund_model::Model;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use database_mysql_seaorm::Dao;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::{DbErr, EntityTrait};

pub async fn find_all() -> Result<Vec<Model>, DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    fund_model::Entity::find().all(&dao.connection).await
}

pub async fn find_by_exchange(exchange: &str) -> Result<Vec<Model>, DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    fund_model::Entity::find()
        .filter(fund_model::Column::Exchange.eq(exchange))
        .all(&dao.connection)
        .await
}
