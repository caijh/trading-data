use crate::exchange::exchange_job::SyncStocksJob;
use crate::exchange::exchange_svc;
use application_core::lang::runnable::Runnable;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::spawn;
use tracing::info;

#[derive(Serialize, Deserialize)]
struct MarketStatusParams {
    pub stock_code: String,
}

#[get("/exchange/:exchange/time")]
async fn exchange_current_time(Path(exchange): Path<String>) -> impl IntoResponse {
    let r = exchange_svc::get_current_time(&exchange).await;
    RespBody::result(&r)
}

#[get("/market/status")]
async fn get_market_status_by_stock_code(
    Query(params): Query<MarketStatusParams>,
) -> impl IntoResponse {
    info!("Get market status by stock_code {}", params.stock_code);
    let r = exchange_svc::get_market_status_by_stock_code_from_cache(&params.stock_code).await;
    RespBody::result(&r)
}

/**
 * 同步指定交易所的股票数据。
 *
 * # 参数
 * `exchange`: 代表需要同步的交易所的名称, sh or sz.
 *
 * # 返回值
 * 实现了 `IntoResponse` 的一个类型，通常用于构建HTTP响应。
 */
#[get("/exchange/stock/sync/:exchange")]
async fn sync(Path(exchange): Path<String>) -> impl IntoResponse {
    spawn(async {
        let job = SyncStocksJob { exchange };
        job.run().await;
    });

    RespBody::<()>::success_info("Sync Stocks in background")
}
