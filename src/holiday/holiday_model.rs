use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};

/// 休市日期
#[derive(Serialize, Deserialize, DeriveEntityModel, Debug, Clone)]
#[sea_orm(table_name = "market_holiday")]
pub struct Model {
    /// id 为日期, 格式为 yyyyMMdd
    #[sea_orm(primary_key)]
    pub id: u64,
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
