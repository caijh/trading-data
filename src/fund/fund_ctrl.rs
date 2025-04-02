use crate::fund::fund_job::SyncFundPriceJob;
use application_core::lang::runnable::Runnable;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::response::IntoResponse;
use tokio::spawn;

/// 同步所有指数中股票的价格
#[get("/fund/sync/price")]
pub async fn sync_fund_price() -> impl IntoResponse {
    spawn(async {
        let job = SyncFundPriceJob;

        job.run().await;
    });

    RespBody::<()>::success_info("Sync Fund prices in background")
}
