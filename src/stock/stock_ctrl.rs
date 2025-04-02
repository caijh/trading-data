use crate::stock::stock_svc;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::extract::Query;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize)]
struct StockParams {
    code: String,
}

/// 获取股票基本信息
#[get("/stock")]
async fn stock_base_info(Query(params): Query<StockParams>) -> impl IntoResponse {
    info!("Get stock base info, code = {}", params.code);
    let r = stock_svc::get_stock(&params.code).await;
    RespBody::result(&r).response()
}

/// 获取股票当前价格
#[get("/stock/price")]
async fn stock_price(Query(params): Query<StockParams>) -> impl IntoResponse {
    info!("Query stock price, code = {}", params.code);
    let r = stock_svc::get_stock_price(&params.code).await;
    RespBody::result(&r).response()
}

/// 获取股票日线价格
#[get("/stock/price/daily")]
async fn stock_daily_price(Query(params): Query<StockParams>) -> impl IntoResponse {
    let r = stock_svc::get_stock_daily_price(&params.code, true).await;
    RespBody::result(&r).response()
}

/// 同步股票日价格数据
///
/// # Arguments
///
/// * `Query(params)` - 一个从查询字符串中解析出来的股票参数对象，包含股票代码
///
/// # Returns
///
/// 返回一个实现了`IntoResponse` trait的对象，用于构建HTTP响应
#[get("/stock/price/daily/sync")]
async fn sync_stock_daily_price(Query(params): Query<StockParams>) -> impl IntoResponse {
    let r = stock_svc::sync_stock_daily_price(&params.code).await;
    RespBody::result(&r).response()
}
