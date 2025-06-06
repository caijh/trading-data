use crate::exchange::exchange_model::Exchange;
use crate::holiday::holiday_model::{Model, create_holiday_model};
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Months, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rand::{Rng, rng};
use scraper::{ElementRef, Html, Selector};
use std::error::Error;
use util::request::Request;

#[async_trait]
pub trait HolidayApi {
    async fn get_holidays(&self) -> Result<Vec<Model>, Box<dyn Error>>;
}

#[async_trait]
impl HolidayApi for Exchange {
    async fn get_holidays(&self) -> Result<Vec<Model>, Box<dyn Error>> {
        match self {
            Exchange::SSE | Exchange::SZSE => get_china_stock_holiday(self).await,
            Exchange::HKEX => get_holiday_from_gov_hk(self).await,
            Exchange::NASDAQ => get_holiday_from_nasdaq(self).await,
        }
    }
}

async fn get_china_stock_holiday(exchange: &Exchange) -> Result<Vec<Model>, Box<dyn Error>> {
    let utc = Utc
        .with_ymd_and_hms(Utc::now().year(), 1, 1, 0, 0, 0)
        .unwrap();
    let month = utc.with_timezone(&exchange.time_zone());
    let mut vec = Vec::new();
    let mut i = 0;
    while i < 12 {
        let month = month.checked_add_months(Months::new(i)).unwrap();
        let r = get_holiday_from_sz(exchange, &month).await?;
        for x in r {
            vec.push(x)
        }
        i += 1;
    }
    Ok(vec)
}

async fn get_holiday_from_sz(
    exchange: &Exchange,
    month: &DateTime<Tz>,
) -> Result<Vec<Model>, Box<dyn Error>> {
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let base_url = environment
        .get_property::<String>("stock.api.sz.baseurl")
        .unwrap();
    let month = month.format("%Y-%m").to_string();
    let url = format!(
        "{}/api/report/exchange/onepersistenthour/monthList?month={}&random={}",
        base_url,
        month,
        rng().random::<f64>(),
    );
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let json: serde_json::Value = response.json().await?;
    let data = json.get("data").unwrap().as_array().unwrap();
    let mut vec = Vec::new();
    for h in data {
        let bz = h.get("jybz").unwrap().as_str().unwrap();
        if bz == "1" {
            continue;
        }
        let date = h.get("jyrq").unwrap().as_str().unwrap();
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d");
        if date.is_err() {
            continue;
        }
        let date = date.unwrap();
        let xh = h
            .get("zrxh")
            .unwrap()
            .as_number()
            .unwrap()
            .as_u64()
            .unwrap();
        if xh == 7 || xh == 1 {
            continue;
        }
        let id = format!("{}{}", date.format("%Y%m%d"), exchange.int_code());
        vec.push(Model {
            id: id.parse::<u64>().unwrap(),
            year: date.year() as u16,
            month: date.month() as u8,
            day: date.day() as u8,
        })
    }
    Ok(vec)
}

async fn get_holiday_from_nasdaq(exchange: &Exchange) -> Result<Vec<Model>, Box<dyn Error>> {
    let url = "https://www.nasdaq.com/market-activity/stock-market-holiday-schedule";
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    // Parse the HTML document
    let document = Html::parse_document(&body);

    // Define a selector for the table rows within the holiday schedule table
    let row_selector = Selector::parse("div.nsdq_table--responsive table tbody tr").unwrap();

    let mut vec = Vec::new();
    // Iterate over each row in the table
    for row in document.select(&row_selector) {
        // Define a selector for the table cells within each row
        let cell_selector = Selector::parse("td").unwrap();
        // Collect the text content of each cell
        let cells: Vec<String> = collect_cells(&row, &cell_selector);

        let date = cells[1].clone();
        // Define the format to match "May 26"
        let format = "%B %d";

        // Assume the current year
        let current_year = Utc::now().year();

        // Parse the month and day, then construct the full date
        let parsed_date = NaiveDate::parse_from_str(
            &format!("{} {}", date, current_year),
            &format!("{} %Y", format),
        )?;
        let id = format!("{}{}", parsed_date.format("%Y%m%d"), exchange.int_code());
        let holiday = create_holiday_model(
            id.parse::<u64>().unwrap(),
            parsed_date.year() as u16,
            parsed_date.month() as u8,
            parsed_date.day() as u8,
        );
        vec.push(holiday);
    }

    Ok(vec)
}

async fn get_holiday_from_gov_hk(exchange: &Exchange) -> Result<Vec<Model>, Box<dyn Error>> {
    let utc = Utc::now();
    let year = utc.with_timezone(&exchange.time_zone()).year();
    let url = format!("https://www.gov.hk/sc/about/abouthk/holiday/{}.htm", year);
    let client = Request::client().await;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    // Parse the HTML document
    let document = Html::parse_document(&body);

    let row_selector = Selector::parse("section.blockItem table:first-of-type tbody tr").unwrap();

    let mut vec = Vec::new();
    for row in document.select(&row_selector) {
        let cell_selector = Selector::parse("td").unwrap();
        let cells = collect_cells(&row, &cell_selector);

        let date = cells[1].clone();
        if date.is_empty() {
            continue;
        }

        let format = "%m月%d日";
        let parsed_date =
            NaiveDate::parse_from_str(&format!("{}年{}", year, date), &format!("%Y年{}", format))?;
        let id = format!("{}{}", parsed_date.format("%Y%m%d"), exchange.int_code());
        let holiday = create_holiday_model(
            id.parse::<u64>().unwrap(),
            parsed_date.year() as u16,
            parsed_date.month() as u8,
            parsed_date.day() as u8,
        );
        vec.push(holiday);
    }

    Ok(vec)
}

fn collect_cells(row: &ElementRef, cell_selector: &Selector) -> Vec<String> {
    let cells: Vec<String> = row
        .select(&cell_selector)
        .map(|cell| cell.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .collect();
    cells
}
