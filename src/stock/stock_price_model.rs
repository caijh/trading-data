use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, DeriveEntityModel)]
#[sea_orm(table_name = "stock_daily_price")]
/// 表示股票每日价格信息的结构体
pub struct Model {
    #[sea_orm(primary_key)]
    /// 股票代码
    pub code: String,
    /// 交易日期
    #[sea_orm(primary_key)]
    pub date: u64,
    /// 当日开盘价
    pub open: BigDecimal,
    /// 当日收盘价
    pub close: BigDecimal,
    /// 当日最高价
    pub high: BigDecimal,
    /// 当日最低价
    pub low: BigDecimal,
    /// 当日成交量，可能为空
    pub volume: Option<BigDecimal>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
