use crate::exchange::exchange_model::Exchange;
use crate::holiday::holiday_api::HolidayApi;
use crate::holiday::{holiday_dao, holiday_model};
use application_beans::factory::bean_factory::BeanFactory;
use application_context::context::application_context::APPLICATION_CONTEXT;
use chrono::{DateTime, Datelike, Local};
use database_mysql_seaorm::Dao;
use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::str::FromStr;
use tracing::info;

#[derive(Serialize, Deserialize, Clone)]
pub struct HolidayQueryResult {
    pub is_holiday: bool,
}

pub async fn is_holiday(
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

pub async fn today_is_holiday(exchange: &str) -> Result<bool, Box<dyn Error>> {
    let now = Local::now();
    let exchange = Exchange::from_str(exchange)?;
    is_holiday(&now, &exchange).await
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
                    holidays.push(holiday_model::ActiveModel {
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

    let application_context = APPLICATION_CONTEXT.read().await;
    let dao = application_context.get_bean_factory().get::<Dao>();
    holiday_model::Entity::insert_many(holidays)
        .on_empty_do_nothing()
        .exec(&dao.connection)
        .await?;
    Ok(())
}
