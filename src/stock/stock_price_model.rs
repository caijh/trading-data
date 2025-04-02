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
    /// 当日成交金额，可能为空
    pub amount: Option<BigDecimal>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub trait KLine {
    fn is_up(&self) -> bool;
    fn is_down(&self) -> bool;

    fn get_real_body(&self) -> BigDecimal;
    fn get_lower_shadow(&self) -> BigDecimal;
    fn get_upper_shadow(&self) -> BigDecimal;

    fn get_middle_price(&self) -> BigDecimal;

    fn is_no_trade(&self) -> bool;
}

impl KLine for Model {
    fn is_up(&self) -> bool {
        self.close >= self.open
    }

    fn is_down(&self) -> bool {
        self.close < self.open
    }

    fn get_real_body(&self) -> BigDecimal {
        if self.is_up() {
            self.close.clone() - self.open.clone()
        } else {
            self.open.clone() - self.close.clone()
        }
    }

    fn get_lower_shadow(&self) -> BigDecimal {
        if self.is_up() {
            self.open.clone() - self.low.clone()
        } else {
            self.close.clone() - self.low.clone()
        }
    }

    fn get_upper_shadow(&self) -> BigDecimal {
        if self.is_up() {
            self.high.clone() - self.close.clone()
        } else {
            self.high.clone() - self.open.clone()
        }
    }

    fn get_middle_price(&self) -> BigDecimal {
        (self.open.clone() + self.close.clone()) / BigDecimal::from(2)
    }

    fn is_no_trade(&self) -> bool {
        self.get_lower_shadow() + self.get_real_body() + self.get_upper_shadow()
            == BigDecimal::from(0)
    }
}
