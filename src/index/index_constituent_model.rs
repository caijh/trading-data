use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "index_constituent")]
/// 指数成分股
pub struct Model {
    /// 指数
    #[sea_orm(primary_key)]
    pub index_code: String,
    /// 股票代码
    #[sea_orm(primary_key)]
    pub stock_code: String,
    /// 股票名称
    pub stock_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncIndexConstituents {
    pub added: Vec<Model>,
    pub removed: Vec<Model>,
    pub old: Vec<Model>,
}
