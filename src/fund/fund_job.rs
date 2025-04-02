use crate::fund::fund_svc;
use crate::stock::stock_svc::sync_stock_daily_price;
use application_core::lang::runnable::Runnable;
use async_trait::async_trait;
use rand::{Rng, rng};
use redis::Commands;
use redis_io::Redis;
use tracing::info;

pub struct SyncFundPriceJob;

#[async_trait]
impl Runnable for SyncFundPriceJob {
    async fn run(&self) {
        let seconds = rng().random_range(1..10);
        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

        let client = Redis::get_client();
        let mut con = client.get_connection().unwrap();
        let key = "Sync:Fund:Price".to_string();
        let value = con.get::<&str, Option<String>>(&key).unwrap();

        match value {
            None => {
                con.set_ex::<&str, &str, String>(&key, "doing", 3600)
                    .unwrap();
                info!("SyncFundPriceJob run ...");
                let funds = fund_svc::find_all().await.unwrap();
                for fund in funds {
                    let _ = sync_stock_daily_price(&fund.code).await;
                }
                info!("SyncFundPriceJob end success");
                let _ = con.del::<&str, i32>(&key);
            }
            Some(_value) => {
                info!("Job is running")
            }
        }
    }
}
