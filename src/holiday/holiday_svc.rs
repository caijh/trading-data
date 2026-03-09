use crate::exchange::exchange_model::Exchange;
use crate::holiday::holiday_api::HolidayApi;
use crate::holiday::holiday_dao;
use crate::holiday::holiday_model::ActiveModel;
use application_cache::CacheManager;
use chrono::{DateTime, Datelike, Local};
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

#[derive(Serialize, Deserialize, Clone)]
pub struct HolidayQueryResult {
    pub is_holiday: bool,
}

async fn date_is_holiday(
    date: &DateTime<Local>,
    exchange: &Exchange,
) -> Result<bool, Box<dyn Error>> {
    let date = date.with_timezone(&exchange.time_zone());

    if date.weekday().number_from_monday() == 6 || date.weekday().number_from_monday() == 7 {
        return Ok(true);
    }

    let market_holiday = holiday_dao::get_holiday(&exchange, &Local::now()).await?;
    match market_holiday {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

async fn _today_is_holiday(exchange: &str) -> Result<bool, Box<dyn Error>> {
    let now = Local::now();
    let exchange = Exchange::from_str(exchange)?;
    date_is_holiday(&now, &exchange).await
}

async fn get_holiday_status_cache(key: &str) -> Option<Result<bool, Box<dyn Error>>> {
    let is_holiday = CacheManager::get_from("HolidayStatus", key).await;
    if is_holiday.is_some() {
        let is_holiday = is_holiday.unwrap();
        let is_holiday = is_holiday
            .parse::<bool>()
            .map_err(|e| Box::new(e) as Box<dyn Error>);
        return Some(is_holiday);
    }
    None
}

pub async fn is_holiday(exchange: &str) -> Result<bool, Box<dyn Error>> {
    let key = format!("HolidayStatus:{}", exchange);
    if let Some(value) = get_holiday_status_cache(&key).await {
        return value;
    }

    let holiday_status = _today_is_holiday(exchange).await?;
    let holiday_status_str = holiday_status.to_string();
    CacheManager::set_to(
        "HolidayStatus",
        &key,
        &holiday_status_str,
        Duration::from_secs(3600),
    )
    .await;
    Ok(holiday_status)
}

pub async fn sync_holidays() -> Result<(), Box<dyn Error>> {
    let dates = holiday_dao::get_all_holiday().await?;
    let dates = dates.into_iter().map(|date| date.id).collect::<Vec<_>>();

    let mut holidays = Vec::new();
    for exchange in Exchange::VALUES {
        info!("Sync {:?} holidays", exchange.as_ref());
        let result = exchange.get_holidays().await;
        if let Ok(vec) = result {
            vec.iter().for_each(|date| {
                if !dates.contains(&date.id) {
                    holidays.push(ActiveModel {
                        id: Set(date.id),
                        year: Set(date.year),
                        month: Set(date.month),
                        day: Set(date.day),
                    });
                }
            });
        }
    }
    if holidays.is_empty() {
        return Ok(());
    }

    holiday_dao::save_holidays(holidays).await?;
    Ok(())
}
