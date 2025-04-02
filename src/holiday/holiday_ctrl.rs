use crate::holiday::holiday_job::SyncHolidayJob;
use crate::holiday::holiday_svc;
use application_core::lang::runnable::Runnable;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::extract::Query;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct HolidayParams {
    exchange: String,
}

/// 检查今天是否为假日。如果今天是假日，返回`true`；否则，返回`false`.
///
/// 这个函数首先获取当前的本地时间，然后用`is_holiday`函数来检查这个日期是否为假日。
///
/// 最后，将结果封装在`RespBody`中，并作为响应返回。
#[get("/holiday/today")]
async fn today_is_holiday(Query(params): Query<HolidayParams>) -> impl IntoResponse {
    let r = holiday_svc::today_is_holiday(&params.exchange).await;

    RespBody::result(&r).response()
}

/// 定义一个异步函数sync，返回类型为IntoResponse的实现
/// 该函数首先调用sync_holidays异步方法获取数据
/// 然后将结果转换为RespBody，并构建一个响应对象
#[get("/holiday/sync")]
async fn sync() -> impl IntoResponse {
    let job = SyncHolidayJob;

    job.run().await;

    RespBody::<()>::success_info("Sync Done")
}
