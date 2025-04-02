use crate::exchange::exchange_model::Exchange;
use crate::holiday::holiday_model;
use crate::holiday::holiday_model::Model;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use chrono::{DateTime, Local};
use database_mysql_seaorm::Dao;
use sea_orm::{DbErr, EntityTrait};

pub async fn get_all_holiday() -> Result<Vec<Model>, DbErr> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    holiday_model::Entity::find().all(&dao.connection).await
}

pub async fn get_holiday(
    exchange: &Exchange,
    date: &DateTime<Local>,
) -> Result<Option<Model>, DbErr> {
    let date = date.with_timezone(&exchange.time_zone());
    let date = format!("{}{}", date.format("%Y%m%d"), exchange.int_code());
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    holiday_model::Entity::find_by_id(date.parse::<u64>().unwrap())
        .one(&dao.connection)
        .await
}
