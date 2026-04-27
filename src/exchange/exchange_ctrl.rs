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

/// 市场状态查询请求参数
///
/// 用于通过股票代码查询市场状态的查询参数
#[derive(Serialize, Deserialize)]
struct MarketStatusParams {
    /// 股票代码，用于查询该股票所属市场的交易状态
    pub stock_code: String,
}

/// 获取交易所列表
///
/// 处理对 `/exchange/list` 路径的 GET 请求，返回系统中所有可用交易所的列表。
/// 该接口主要用于前端或客户端获取支持的交易所信息，便于后续操作选择。
///
/// # 示例
///
/// ```
/// GET /exchange/list
/// ```
///
/// # Returns
///
/// * `impl IntoResponse` - 返回一个实现了 `IntoResponse` trait 的类型，用于生成 HTTP 响应
///
/// # 返回数据
///
/// 返回包含交易所标识符的数组，例如：`["sh", "sz"]`
#[get("/exchange/list")]
async fn exchange_list() -> impl IntoResponse {
    let exchanges = Exchange::VALUES
        .iter()
        .map(|e| e.as_ref().to_string())
        .collect::<Vec<_>>();
    RespBody::success(&exchanges)
}

/// 获取指定交易所的当前时间
///
/// 处理对 `/exchange/{exchange}/time` 路径的 GET 请求，返回指定交易所的当前时间。
/// 该接口可用于客户端与交易所时间同步，或用于时间相关的业务逻辑判断。
///
/// # 参数
///
/// * `exchange` - 交易所代码，通过 URL 路径参数传递，例如 `SSE`（上海证券交易所）或 `SZSE`（深圳证券交易所）
///
/// # 示例
///
/// ```
/// GET /exchange/SSE/time
/// ```
///
/// # Returns
///
/// * `impl IntoResponse` - 返回一个实现了 `IntoResponse` trait 的类型，用于生成 HTTP 响应
///
/// # 返回数据
///
/// 返回指定交易所的当前时间信息
#[get("/exchange/{exchange}/time")]
async fn exchange_current_time(Path(exchange): Path<String>) -> impl IntoResponse {
    let r = exchange_svc::get_exchange_current_time(&exchange).await;
    RespBody::result(&r)
}

/// 获取指定交易所的市场状态
///
/// 处理对 `/exchange/{exchange}/market/status` 路径的 GET 请求，返回指定交易所的当前市场状态信息。
/// 市场状态包括：交易时段、休市状态、开盘/收盘时间等关键信息。
/// 数据从缓存中获取，确保高响应性能。
///
/// # 参数
///
/// * `exchange` - 交易所代码，通过 URL 路径参数传递，例如 `SSE`（上海证券交易所）或 `SZSE`（深圳证券交易所）
///
/// # 示例
///
/// ```
/// GET /exchange/SSE/market/status
/// ```
///
/// # Returns
///
/// * `impl IntoResponse` - 返回一个实现了 `IntoResponse` trait 的类型，用于生成 HTTP 响应
///
/// # 返回数据
///
/// 返回包含市场状态的详细信息，如：
/// - 是否开市
/// - 当前时段（开盘前、交易中、收盘后、休市）
/// - 今日开盘时间和收盘时间
///
/// # Remarks
///
/// 使用 `Path` 参数来捕获 URL 中的 `exchange` 部分，以便于获取特定交易所的信息。
/// 通过调用 `exchange_svc::get_exchange_market_status` 函数来获取市场状态信息。
/// 最后使用 `RespBody::result` 来根据查询结果构建 HTTP 响应。
#[get("/exchange/{exchange}/market/status")]
async fn get_market_status(Path(exchange): Path<String>) -> impl IntoResponse {
    let r = exchange_svc::get_exchange_market_status(&exchange).await;
    RespBody::result(&r)
}

/// 根据股票代码获取市场状态
///
/// 处理对 `/market/status` 路径的 GET 请求，通过股票代码查询其所属市场的当前状态。
/// 该接口适用于根据具体股票代码查询市场状态的场景，无需预先知道股票代码所属的交易所。
///
/// # 参数
///
/// * `params` - 查询参数，包含 `stock_code` 字段，通过 URL 查询字符串传递
///
/// # 示例
///
/// ```
/// GET /market/status?stock_code=600000
/// ```
///
/// # Returns
///
/// * `impl IntoResponse` - 返回一个实现了 `IntoResponse` trait 的类型，用于生成 HTTP 响应
///
/// # 返回数据
///
/// 返回该股票代码所属市场的状态信息，与 `get_market_status` 返回格式相同
///
/// # 日志
///
/// 请求时会记录日志，输出查询的股票代码
#[get("/market/status")]
async fn get_market_status_by_stock_code(
    Query(params): Query<MarketStatusParams>,
) -> impl IntoResponse {
    info!("Get market status by stock_code {}", params.stock_code);
    let r = exchange_svc::get_stock_market_status(&params.stock_code).await;
    RespBody::result(&r)
}

/// 同步指定交易所的股票数据
///
/// 处理对 `/exchange/stock/sync/{exchange}` 路径的 GET 请求，触发指定交易所的股票数据同步任务。
/// 该接口用于手动触发股票数据同步，适用于数据补录、定时任务触发等场景。
///
/// # 参数
///
/// * `exchange` - 交易所代码，通过 URL 路径参数传递，例如 `SSE`（上海证券交易所）或 `SZSE`（深圳证券交易所）
///
/// # 示例
///
/// ```
/// GET /exchange/stock/sync/SSE
/// ```
///
/// # Returns
///
/// * `impl IntoResponse` - 返回一个实现了 `IntoResponse` trait 的类型，用于生成 HTTP 响应
///
/// # 返回数据
///
/// 立即返回确认信息，表示同步任务已启动。实际的数据同步在后台异步执行。
///
/// # 异步处理
///
/// 该接口采用异步处理方式：
/// 1. 接收到请求后立即返回确认信息
/// 2. 在后台启动 `SyncStocksJob` 任务进行数据同步
/// 3. 不阻塞 HTTP 响应，提高接口响应速度
///
/// # 注意事项
///
/// - 同步过程可能耗时较长，取决于交易所股票数量和网络状况
/// - 建议避免频繁调用，以免对数据源造成压力
/// - 可通过日志或监控工具查看同步进度和结果
#[get("/exchange/stock/sync/{exchange}")]
async fn sync(Path(exchange): Path<String>) -> impl IntoResponse {
    spawn(async {
        let job = SyncStocksJob { exchange };
        job.run().await;
    });

    RespBody::<()>::success_info("Sync Stocks in background")
}
