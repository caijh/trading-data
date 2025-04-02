use application_web::response::RespBody;
use application_web_macros::get;
use axum::response::IntoResponse;

use crate::currency::currency_svc;

#[get("/currency/rate")]
pub async fn get_rate() -> impl IntoResponse {
    let r = currency_svc::get_rate().await;

    RespBody::result(&r).response()
}
