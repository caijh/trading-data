use crate::exchange::exchange_model::Exchange;
use crate::index::index_job::{SyncAllIndexStockPriceJob, SyncIndexStocksJob};
use crate::index::index_svc;
use application_core::lang::runnable::Runnable;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::spawn;

#[derive(Serialize, Deserialize)]
struct GetIndexParams {
    pub exchange: Option<String>,
}

/// 获取股票指数信息
///
/// 如果没有提供交易所参数，将返回所有交易所的股票指数信息；否则，返回指定交易所的股票指数信息
///
/// 参数:
/// - Query(params): 一个查询参数对象，包含可选的交易所信息
///
/// 返回:
/// - impl IntoResponse
#[get("/index/list")]
pub async fn get_all_index(Query(params): Query<GetIndexParams>) -> impl IntoResponse {
    // 根据查询参数中的交易所信息，转换为Exchange枚举类型，如果未提供则为None

    let r = if params.exchange.is_none() {
        index_svc::find_all_stock_index().await
    } else {
        let exchange = Exchange::from_str(&params.exchange.unwrap()).unwrap();
        index_svc::find_stock_index_by_exchange(&exchange).await
    };

    RespBody::result(&r)
}

/// 获取指数的成分股
///
/// # Arguments
///
/// * `code`:
///
/// returns: Result<Vec<IndexConstituent, Global>, Box<dyn Error, Global>>
///
/// # Examples
///
/// ```
///
/// ```
#[get("/index/:code/stocks")]
pub async fn get_stocks(Path(code): Path<String>) -> impl IntoResponse {
    let r = index_svc::get_constituent_stocks(&code).await;

    RespBody::result(&r).response()
}

/// 后台同步指定指数的股票信息
#[get("/index/sync/:code")]
pub async fn sync(Path(code): Path<String>) -> impl IntoResponse {
    let r = index_svc::sync_constituents(&code).await;

    RespBody::result(&r).response()
}

/// 同步所有指数的股票信息
///
/// 该函数通过异步任务启动一个后台作业，用于同步指数股票信息，
/// 并立即返回成功信息给前端，不会等待同步任务完成。
///
/// # Returns
///
/// 返回一个实现IntoResponse的类型，通常是一个HTTP响应，
/// 表示后台同步任务已成功启动。
#[get("/index/sync")]
pub async fn sync_all() -> impl IntoResponse {
    spawn(async {
        let job = SyncIndexStocksJob;
        job.run().await;
    });

    RespBody::<()>::success_info("Sync index Stocks in background")
}

#[derive(Serialize, Deserialize)]
struct IndexStockPriceSyncParams {
    code: Option<String>,
}

/// 同步所有指数中股票的价格
#[get("/index/sync/price")]
pub async fn sync_index_stock_price(
    Query(params): Query<IndexStockPriceSyncParams>,
) -> impl IntoResponse {
    spawn(async {
        let job = SyncAllIndexStockPriceJob { code: params.code };

        job.run().await;
    });

    RespBody::<()>::success_info("Sync index Stocks prices in background")
}
