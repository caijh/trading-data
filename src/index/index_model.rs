use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "stock_index")]
/// 指数
pub struct Model {
    /// 指数代码
    #[sea_orm(primary_key)]
    pub code: String,
    /// 指数名称
    pub name: String,
    /// 交易所
    pub exchange: String,
    pub index_code: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
