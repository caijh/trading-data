use crate::exchange::exchange_model::Exchange;
use crate::fund::fund_dao;
use crate::fund::fund_model::Model;
use std::error::Error;
use std::str::FromStr;

pub async fn find_all() -> Result<Vec<Model>, Box<dyn Error>> {
    let funds = fund_dao::find_all().await?;
    Ok(funds)
}

pub async fn find_by_exchange(exchange: &str) -> Result<Vec<Model>, Box<dyn Error>> {
    let exchange = Exchange::from_str(exchange)?;
    let funds = fund_dao::find_by_exchange(exchange.as_ref()).await?;
    Ok(funds)
}
