use actix_web::{HttpMessage, HttpRequest, HttpResponse, post, web};
use serde::Deserialize;
use sqlx::PgPool;

use crate::middleware::auth_middleware::Claims;

#[derive(Deserialize)]
pub struct OrderData {
    side: String,
    qty: i32,
    price: i32,
}
#[post("/api/order")]
pub async fn fetch_order(
    req: HttpRequest,
    pool: web::Data<Option<PgPool>>,
    req_body: web::Json<OrderData>,
) -> HttpResponse {
    let pool = match pool.get_ref() {
        Some(p) => p,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(serde_json::json!({"fail_reason": "database not connected"}));
        }
    };

    let exts = req.extensions();
    let claims = exts.get::<Claims>().unwrap();

    // check if the order can be placed
    // check if the user has the required assets

    // if req_body.size == SELL check if qty >= balance_btc
    // if req_body.side == BUY check if qty >= balance_inr

    let result = sqlx::query(
        "INSERT INTO orders (user_id, side, qty, price, status)
     VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(claims.id)
    .bind(&req_body.side)
    .bind(&req_body.qty)
    .bind(&req_body.price)
    .bind("pending")
    .execute(pool)
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status": "order placed"})),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"fail_reason": e.to_string()})),
    }
}
