use crate::domain::order::{Order, OrderRequest};
use crate::matching_engine::orderbook::OrderBook;
use crate::middleware::auth_middleware::Claims;
use crate::trading_engine::engine::settle_trades;
//
use actix_web::{HttpMessage, HttpRequest, HttpResponse, post, web};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

pub fn ConvertToOrder(req: &OrderRequest, user_id: i32) -> Order {
    Order {
        user_id,
        side: req.side.clone(),
        qty: req.qty,
        price: req.price,
    }
}

#[post("/api/order")]
pub async fn fetch_order(
    req: HttpRequest,
    orderbook: web::Data<Arc<Mutex<OrderBook>>>,
    pool: web::Data<Option<PgPool>>,
    req_body: web::Json<OrderRequest>,
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

    if req_body.qty <= 0 {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!(" Qty of asset must be valid "));
    }
    if req_body.price <= 0 {
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

    // call the matching engine
    let order_convert = ConvertToOrder(&req_body, claims.id);
    let mut ob = orderbook.lock().unwrap();

    let trade_list = ob.engine(pool, order_convert).await;
    // ord_response Returns a TradeList
    match settle_trades(claims.id, trade_list, pool).await {
        Ok(balances) => HttpResponse::Ok().json(balances),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"fail_reason": e.to_string()})),
    }
}
#[post("/api/orderbook")]
pub async fn display_orderbook(orderbook: web::Data<Arc<Mutex<OrderBook>>>) -> HttpResponse {
    let ob = orderbook.lock().unwrap();
    HttpResponse::Ok().json(serde_json::json!({"orderbook": ob.display_orderbook()}))
}
