use actix_web::{HttpMessage, HttpRequest, HttpResponse, post, web};
use serde::Deserialize;
use sqlx::PgPool;

use crate::auth::common::AuthData;
use crate::middleware::auth_middleware::Claims;

#[derive(Deserialize)]
pub struct OrderData {
    side: String,
    qty: i32,
    price: i32,
}
#[derive(Deserialize)]
pub struct OrderDataStore {
    user_id: i32,
    side: String,
    qty: i32,
    price: i32,
}
#[post("/api/order")]
pub async fn fetch_order(
    req: HttpRequest,
    _pool: web::Data<Option<PgPool>>,
    _req_body: web::Json<AuthData>,
) -> HttpResponse {
    let exts = req.extensions();
    let claims = exts.get::<Claims>().unwrap();
    println!("id :{}", claims.id);
    HttpResponse::Ok().finish()
}
