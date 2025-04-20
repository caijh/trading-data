use database_mysql_seaorm::Dao;
use std::error::Error;
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use crate::stock::{stock_model, stock_price_model};

/// 异步获取指定股票的价格信息
///
/// # Arguments
/// * `stock` - 一个引用，指向股票模型实例，用于获取股票代码
///
/// # Returns
/// 返回一个结果，包含股票价格信息的向量，如果获取失败，则包含错误信息
///
/// # Remarks
/// 该函数通过应用程序上下文获取数据访问对象（DAO），并使用它来查询数据库中与指定股票代码关联的所有股票价格信息
/// 价格信息按照日期升序排列，以便于时间序列分析或显示
pub async fn get_stock_prices(stock: &stock_model::Model) -> Result<Vec<stock_price_model::Model>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    let prices = stock_price_model::Entity::find()
        .filter(stock_price_model::Column::Code.eq(&stock.code))
        .order_by_asc(stock_price_model::Column::Date)
        .all(&dao.connection)
        .await?;
    Ok(prices)
}
