use std::error::Error;

use crate::currency::currency_api::CurrencyApi;
use crate::currency::currency_model::CurrencyRate;
use crate::exchange::exchange_model::Exchange;

pub async fn get_rate() -> Result<Vec<CurrencyRate>, Box<dyn Error>> {
    Exchange::SSE.get_rate().await
}
