use crate::fund::fund_model;
use crate::fund::fund_model::Model;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use database_mysql_seaorm::Dao;
use sea_orm::EntityTrait;
use std::error::Error;

pub async fn find_all() -> Result<Vec<Model>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let funds = fund_model::Entity::find().all(&dao.connection).await?;
    Ok(funds)
}
