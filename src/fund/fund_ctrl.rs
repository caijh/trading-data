use crate::fund::fund_job::SyncFundPriceJob;
use crate::fund::fund_svc;
use application_core::lang::runnable::Runnable;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::extract::Path;
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

/// 通过交易所名称获取基金信息
///
/// 该路由接收一个包含交易所名称的URL参数，并返回该交易所的资金信息
/// 使用`Path`提取路由参数中的`exchange`字段，用于查询资金信息
///
/// # Parameters
/// - `Path(exchange)`: 一个从URL路径提取的字符串参数，代表交易所名称
///
/// # Returns
/// - `impl IntoResponse`: 返回一个实现了`IntoResponse` trait的类型，用于构建HTTP响应
///
/// # Note
/// - 使用`fund_svc::find_by_exchange`异步函数查询资金信息，该函数根据交易所名称查询并返回资金记录
/// - 最后，使用`RespBody::result(&r)`来构建响应体，它根据查询结果生成适当的HTTP响应
#[get("/exchange/:exchange/funds")]
pub async fn get_exchange_funds(Path(exchange): Path<String>) -> impl IntoResponse {
    let r = fund_svc::find_by_exchange(&exchange).await;

    RespBody::result(&r)
}
