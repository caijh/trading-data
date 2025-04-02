use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DebtPrice {
    pub current: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub zd: String,
    pub zdf: String,
    pub yc: String,
    pub v: String,
    pub cje: String,
    pub t: String,
}
