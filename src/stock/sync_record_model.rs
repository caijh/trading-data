use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};

/// 表示股票每日价格同步记录的结构体。
#[derive(Debug, Clone, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "stock_daily_price_sync_record")]
pub struct Model {
    /// 股票代码，以字符串形式存储。
    #[sea_orm(primary_key)]
    pub code: String,
    /// 日期，以整型64位有符号数存储，代表自1970年1月1日以来的秒数。
    pub date: u64,
    /// 更新状态，使用特殊序列化方法处理，可以是布尔值或整型。
    pub updated: bool,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
