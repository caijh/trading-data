use crate::debt::debt_api;
use crate::debt::debt_model::DebtPrice;
use std::error::Error;

pub async fn get_debt_price(code: &str) -> Result<DebtPrice, Box<dyn Error>> {
    debt_api::get_debt_price(code).await
}
