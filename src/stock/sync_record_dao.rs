use database_mysql_seaorm::Dao;
use std::error::Error;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use sea_orm::EntityTrait;
use crate::stock::{stock_model, sync_record_model};

/// 获取同步记录
///
/// 本函数通过股票模型中的代码异步获取对应的同步记录模型
/// 主要用于在同步股票数据时，检查数据库中是否存在对应的同步记录
///
/// # 参数
/// * `stock` - 一个引用，指向股票模型实例，包含股票相关信息
///
/// # 返回值
/// 返回一个结果，包含可选的同步记录模型或错误类型
/// 如果找到对应的同步记录，则返回 Some(sync_record_model::Model)，
/// 否则返回 None，如果发生错误，则返回错误类型
pub async fn get_sync_record(stock: &stock_model::Model) -> Result<Option<sync_record_model::Model>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let sync_record = sync_record_model::Entity::find_by_id(&stock.code)
        .one(&dao.connection)
        .await?;
    Ok(sync_record)
}
