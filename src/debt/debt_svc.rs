use crate::debt::debt_api::DebtApi;
use crate::debt::debt_model::DebtPrice;
use crate::exchange::exchange_model::Exchange;
use std::error::Error;

pub async fn get_debt_price(code: &str) -> Result<DebtPrice, Box<dyn Error>> {
    Exchange::SSE.get_debt_price(code).await
}
