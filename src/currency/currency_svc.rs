use std::error::Error;

use crate::currency::currency_api;
use crate::currency::currency_model::CurrencyRate;

pub async fn get_rate() -> Result<Vec<CurrencyRate>, Box<dyn Error>> {
    currency_api::get_rate().await
}
