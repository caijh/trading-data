use crate::token::token_svc;
use application_web::response::RespBody;
use application_web_macros::get;
use axum::response::IntoResponse;

#[get("/token/hkex")]
async fn sync() -> impl IntoResponse {
    let result = token_svc::get_hkex_token_from_website().await;

    RespBody::result(&result).response()
}
