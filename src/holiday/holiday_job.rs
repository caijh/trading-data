use crate::holiday::holiday_svc::sync_holidays;
use application_core::lang::runnable::Runnable;
use async_trait::async_trait;
use tracing::error;

pub struct SyncHolidayJob;

#[async_trait]
impl Runnable for SyncHolidayJob {
    async fn run(&self) {
        let r = sync_holidays().await;
        match r {
            Ok(_) => {}
            Err(e) => {
                error!("Sync holiday error {}", e)
            }
        }
    }
}
