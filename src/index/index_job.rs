use crate::index::index_constituent_model::SyncIndexConstituents;
use crate::index::index_svc::{
    find_all_stock_index, sync_constituent_stocks_daily_price, sync_constituents,
};
use crate::index::{index_constituent_model, index_dao};
use application_context::context::application_context::APPLICATION_CONTEXT;
use application_core::env::property_resolver::PropertyResolver;
use application_core::lang::runnable::Runnable;
use async_trait::async_trait;
use notification::{Notification, NotificationConfig};
use rand::{Rng, rng};
use redis::Commands;
use redis_io::Redis;
use tokio::spawn;
use tracing::info;

pub struct SyncIndexStocksJob;

#[async_trait]
impl Runnable for SyncIndexStocksJob {
    async fn run(&self) {
        info!("SyncIndexStocksJob run ...");
        let indexes = index_dao::find_all().await;
        match indexes {
            Ok(indexes) => {
                for index in indexes {
                    let constituents = sync_constituents(&index.code).await.unwrap();
                    if constituents.old.is_empty() {
                        continue;
                    }
                    spawn(notification_index_stocks_changed(index, constituents));
                }
                info!("SyncIndexStocksJob end success");
            }
            Err(e) => {
                info!("SyncIndexStocksJob end fail {}", e.to_string())
            }
        }
    }
}

async fn notification_index_stocks_changed(
    index: crate::index::index_model::Model,
    sync_index_constituents: SyncIndexConstituents,
) {
    let stocks_add = sync_index_constituents.added;
    let stocks_remove = sync_index_constituents.removed;
    let mut stocks_to_send = Vec::new();
    for stock in stocks_add {
        stocks_to_send.push(stock);
        if stocks_to_send.len() == 10 {
            let _ =
                do_notification_index_stocks_changed(&index, stocks_to_send.clone(), true).await;
            stocks_to_send.clear();
        }
    }
    if !stocks_to_send.is_empty() {
        do_notification_index_stocks_changed(&index, stocks_to_send.clone(), true).await;
    }
    for stock in stocks_remove {
        stocks_to_send.push(stock);
        if stocks_to_send.len() == 10 {
            let _ =
                do_notification_index_stocks_changed(&index, stocks_to_send.clone(), false).await;
            stocks_to_send.clear();
        }
    }
    if !stocks_to_send.is_empty() {
        do_notification_index_stocks_changed(&index, stocks_to_send.clone(), false).await;
    }
}

async fn do_notification_index_stocks_changed(
    index: &crate::index::index_model::Model,
    index_constituents: Vec<index_constituent_model::Model>,
    add: bool,
) {
    if index_constituents.is_empty() {
        return;
    }

    let title = "指数成分股关注-".to_string() + index.name.as_str();
    let mut content = "".to_string();
    let label = if add { "增加" } else { "移除" };
    for stock in index_constituents {
        content
            .push_str(format!("{} {:<5} {}\n", label, stock.stock_name, stock.stock_code).as_str());
    }
    let application_context = APPLICATION_CONTEXT.read().await;
    let environment = application_context.get_environment().await;
    let result = environment.get_property::<NotificationConfig>("notification");
    match result {
        None => {}
        Some(notification_config) => {
            let url = format!(
                "{}/send/user/{}",
                notification_config.url, notification_config.receiver
            );
            Notification::create(&title, &content)
                .send(url.as_str(), notification_config.receiver.as_str())
                .await
        }
    }
}
pub struct SyncAllIndexStockPriceJob;

#[async_trait]
impl Runnable for SyncAllIndexStockPriceJob {
    async fn run(&self) {
        let seconds = rng().random_range(1..10);
        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

        let client = Redis::get_client();
        let mut con = client.get_connection().unwrap();
        let key = "Sync:Index:Price".to_string();
        let value = con.get::<&str, Option<String>>(&key).unwrap();

        match value {
            None => {
                con.set_ex::<&str, &str, String>(&key, "doing", 3600)
                    .unwrap();
                info!("SyncAllIndexStockPriceJob run ...");
                let indexes = find_all_stock_index().await.unwrap();
                for index in indexes {
                    let _ = sync_constituent_stocks_daily_price(&index.code).await;
                }
                info!("SyncAllIndexStockPriceJob end success");
                let _ = con.del::<&str, i32>(&key);
            }
            Some(_value) => {
                info!("Job is running")
            }
        }
    }
}
