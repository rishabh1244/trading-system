use crate::domain::market::MarketData;
use crate::domain::order::{Order, OrderRequest};
use crate::matching_engine::orderbook::OrderBook;
use crate::middleware::auth_middleware::Claims;
use crate::trading_engine::engine::settle_trades;
//
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, post, web};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

pub fn ConvertToOrder(req: &OrderRequest, user_id: i32) -> Order {
    Order {
        order_id: None,
        user_id,
        side: req.side.clone(),
        qty: req.qty,
        price: req.price,
        status: "pending".to_string(),
    }
}

#[post("/api/order")]
pub async fn fetch_order(
    req: HttpRequest,
    orderbook: web::Data<Arc<Mutex<OrderBook>>>,
    market_data: web::Data<Arc<Mutex<MarketData>>>,
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
        let balance_btc: Decimal =
            sqlx::query_scalar("SELECT balance_btc from balances where user_id=$1")
                .bind(claims.id)
                .fetch_one(pool)
                .await
                .unwrap_or(Decimal::ZERO);
        let required_btc = Decimal::from(req_body.qty);

        if balance_btc < required_btc {
            return HttpResponse::InternalServerError().json(serde_json::json!(format!(
                " userId : {} Insufficient Balance :- \n BTC Balance : {} Selling QTY : {}\n",
                claims.id, balance_btc, req_body.qty
            )));
        }
    }

    if req_body.side == "BUY" {
        let balance_inr: Decimal =
            sqlx::query_scalar("SELECT balance_inr from balances where user_id=$1")
                .bind(claims.id)
                .fetch_one(pool)
                .await
                .unwrap_or(Decimal::ZERO);
        let required_inr = Decimal::from(req_body.qty) * Decimal::from(req_body.price);
        if balance_inr < required_inr {
            return HttpResponse::InternalServerError().json(serde_json::json!(format!(
                " userId : {} Insufficient Balance :- \n INR Balance : {} Buying QTY : {}\n",
                claims.id, balance_inr, req_body.qty
            )));
        }
    }

    // call the matching engine
    let order_convert = ConvertToOrder(&req_body, claims.id);

    // lock the orderbook ONLY for the (fast, in-memory) matching step,
    // then release it before the slow database settlement work
    let result = {
        let mut ob = orderbook.lock().unwrap();
        ob.engine(order_convert).await
    };

    // run on_trade for every executed trade
    {
        let mut md = market_data.lock().unwrap();
        for trade in result.trades.trades.iter() {
            md.on_trade(trade);
        }
    }
    // we try to declare lock() for a mutex in a scope {} block scoping , so when we are out of
    // the scope it the lock automatically frees

    // ord_response Returns a EngineResult
    match settle_trades(
        claims.id,
        result.trades,
        result.appends,
        result.fulfilled_ids,
        pool,
    )
    .await
    {
        Ok((balances, new_order_id)) => {
            // stamp the freshly inserted DB order id onto the resting order in memory
            if let Some(id) = new_order_id {
                let mut ob = orderbook.lock().unwrap();
                ob.set_last_resting_id(&req_body.side, id);
            }
            HttpResponse::Ok().json(balances)
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"fail_reason": e.to_string()})),
    }
}
#[get("/api/orderbook")]
pub async fn display_orderbook(orderbook: web::Data<Arc<Mutex<OrderBook>>>) -> HttpResponse {
    let ob = orderbook.lock().unwrap();
    HttpResponse::Ok().json(serde_json::json!({"orderbook": ob.display_orderbook()}))
}
