use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, DeriveEntityModel)]
#[sea_orm(table_name = "fund")]
pub struct Model {
    /// 基金代码
    #[sea_orm(primary_key)]
    pub code: String,
    /// 基金名称
    pub name: String,
    /// 交易所代码
    pub exchange: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
