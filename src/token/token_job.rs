use crate::token::token_svc;
use application_core::lang::runnable::Runnable;
use async_trait::async_trait;
use tracing::{error, info};

pub struct SyncHKEXTokenJob;
#[async_trait]
impl Runnable for SyncHKEXTokenJob {
    async fn run(&self) {
        let r = token_svc::reset_hkex_token().await;
        match r {
            Ok(_) => {
                info!("Sync HKEX token success");
            }
            Err(e) => {
                error!("Sync HKEX token error {}", e)
            }
        }
    }
}
