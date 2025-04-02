use application_web::response::RespBody;
use application_web_macros::get;
use axum::extract::Query;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::debt::debt_svc;

#[derive(Serialize, Deserialize)]
struct DebtParams {
    code: String,
}

/// 获取债券价格信息
///
/// 该函数通过HTTP GET请求获取国债逆回购价格信息，请求路径为"/debt/price"
/// 使用`Query`提取请求参数，参数类型为`DebtParams`
///
/// 参数:
/// - Query(params): 从请求中提取的国债逆回购参数，用于查询特定债务的价格信息
///
/// 返回:
/// - impl IntoResponse: 返回一个实现了`IntoResponse` trait的响应对象
#[get("/debt/price")]
async fn get_debt_price(Query(params): Query<DebtParams>) -> impl IntoResponse {
    let r = debt_svc::get_debt_price(&params.code).await;
    RespBody::result(&r).response()
}
