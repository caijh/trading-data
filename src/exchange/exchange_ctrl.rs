use crate::exchange::exchange_job::SyncStocksJob;
use crate::exchange::exchange_model::Exchange;
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

/// 获取交易所列表
///
/// 该函数处理对/exchange/list路径的GET请求，返回一个交易所列表
///
/// # Returns
///
/// * `impl IntoResponse` - 返回一个实现了IntoResponse trait的类型，用于生成HTTP响应
#[get("/exchange/list")]
async fn exchange_list() -> impl IntoResponse {
    let exchanges = Exchange::VALUES
        .iter()
        .map(|e| e.as_ref().to_string())
        .collect::<Vec<_>>();
    RespBody::success(&exchanges)
}

#[get("/exchange/:exchange/time")]
async fn exchange_current_time(Path(exchange): Path<String>) -> impl IntoResponse {
    let r = exchange_svc::get_current_time(&exchange).await;
    RespBody::result(&r)
}

/// 从缓存中获取指定交易所的市场状态信息
///
/// # Arguments
/// * `exchange` - 一个字符串参数，通过URL路径传递，指定要查询的交易所代码
///
/// # Returns
/// 返回一个实现了IntoResponse trait的类型，用于构建HTTP响应
///
/// # Remarks
/// 使用Path参数来捕获URL中的exchange部分，以便于获取特定交易所的信息
/// 通过调用`exchange_svc::get_market_status_from_cache`函数来从缓存中获取市场状态信息
/// 最后使用`RespBody::result`来根据查询结果构建HTTP响应
#[get("/exchange/:exchange/market/status")]
async fn get_market_status(
    Path(exchange): Path<String>,
) -> impl IntoResponse {
    let r = exchange_svc::get_market_status_from_cache(&exchange).await;
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
