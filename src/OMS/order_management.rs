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

    if (req_body.qty <= 0) {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!(" Qty of asset must be valid "));
    }
    if (req_body.price <= 0) {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!("price of asset must be valid "));
    }

    // check if the order can be placed
    // check if the user has the required assets

    // if req_body.side == SELL check if qty >= balance_btc
    // if req_body.side == BUY check if qty >= balance_inr

    if req_body.side == "SELL" {
        let balance_btc: f64 =
            sqlx::query_scalar("SELECT balance_btc::float8 from balances where user_id=$1")
                .bind(claims.id)
                .fetch_one(pool)
                .await
                .unwrap_or(0.0);
        let required_btc = req_body.qty as f64 * req_body.price as f64;

        if balance_btc < required_btc {
            return HttpResponse::InternalServerError().json(serde_json::json!(format!(
                " userId : {} Insufficient Balance :- \n BTC Balance : {} Selling QTY : {}\n",
                claims.id, balance_btc, req_body.qty
            )));
        }
    }

    if req_body.side == "BUY" {
        let balance_inr: f64 =
            sqlx::query_scalar("SELECT balance_inr::float8 from balances where user_id=$1")
                .bind(claims.id)
                .fetch_one(pool)
                .await
                .unwrap_or(0.0);
        let required_inr = req_body.qty as f64 * req_body.price as f64;
        if balance_inr < required_inr {
            return HttpResponse::InternalServerError().json(serde_json::json!(format!(
                " userId : {} Insufficient Balance :- \n INR Balance : {} Buying QTY : {}\n",
                claims.id, balance_inr, req_body.qty
            )));
        }
    }

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
