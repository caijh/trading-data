use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};

/// 休市时间
#[derive(Serialize, Deserialize, DeriveEntityModel, Debug, Clone)]
#[sea_orm(table_name = "market_time")]
pub struct Model {
    /// id 为日期, 格式为 yyyyMMdd
    #[sea_orm(primary_key)]
    pub id: u64,
    pub exchange: String,
    pub start_time: Time,
    pub end_time: Time,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
