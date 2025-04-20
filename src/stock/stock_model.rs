use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use std::fmt::Display;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

/**
 * 表示股票的结构体。
 *
 * # 属性
 * - `code`：股票代码，唯一标识一只股票。
 * - `name`：股票名称。
 * - `exchange`：股票交易所代码，表明该股票在哪个交易所上市。
 */
#[derive(Debug, Serialize, Deserialize, Clone, DeriveEntityModel)]
#[sea_orm(table_name = "stock")]
pub struct Model {
    /// 股票代码
    #[sea_orm(primary_key)]
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 交易所代码
    pub exchange: String,
    /// 股票类型：Stock/Index/Fund
    pub stock_type: String,
    /// 股票在交易中的代码
    pub stock_code: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockPrice {
    pub code: String,
    pub open: Option<BigDecimal>,
    pub close: BigDecimal,
    pub low: Option<BigDecimal>,
    pub high: Option<BigDecimal>,
    pub volume: Option<BigDecimal>,
    pub amount: Option<BigDecimal>,
    pub time: String,
}

pub enum StockKind {
    Stock,
    Fund,
    Index,
}

impl Display for StockKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            StockKind::Stock => "Stock".to_string(),
            StockKind::Fund => "Fund".to_string(),
            StockKind::Index => "Index".to_string(),
        };
        write!(f, "{}", str)
    }
}
