use crate::stock::stock_svc::sync;
use application_core::lang::runnable::Runnable;
use async_trait::async_trait;
use tracing::{error, info};

pub struct SyncStocksJob {
    pub exchange: String,
}

#[async_trait]
impl Runnable for SyncStocksJob {
    async fn run(&self) {
        info!("SyncStocksJob run ...");
        let result = sync(&self.exchange).await;
        match result {
            Ok(_) => {
                info!("SyncStocksJob end success")
            }
            Err(e) => {
                error!("Sync {} stock error {}", &self.exchange, e);
            }
        }
    }
}
