use crate::domain::market::MarketData;
use crate::domain::order::{Order, OrderRequest};
use crate::matching_engine::orderbook::OrderBook;
use crate::middleware::auth_middleware::Claims;
use crate::trading_engine::engine::settle_trades;
//
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, post, web};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::{Arc, Mutex, MutexGuard};

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

fn lock_book(orderbook: &Arc<Mutex<OrderBook>>) -> Result<MutexGuard<'_, OrderBook>, HttpResponse> {
    orderbook.lock().map_err(|_| {
        HttpResponse::InternalServerError()
            .json(serde_json::json!({"fail_reason": "orderbook lock poisoned"}))
    })
}

fn lock_market(
    market_data: &Arc<Mutex<MarketData>>,
) -> Result<MutexGuard<'_, MarketData>, HttpResponse> {
    market_data.lock().map_err(|_| {
        HttpResponse::InternalServerError()
            .json(serde_json::json!({"fail_reason": "market data lock poisoned"}))
    })
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
    let Some(claims) = exts.get::<Claims>() else {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"fail_reason": "missing auth claims"}));
    };

    if req_body.qty <= 0 {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!(" Qty of asset must be valid "));
    }
    if req_body.price <= 0 {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!("price of asset must be valid "));
    }

    // lock funds atomically: check the balance AND deduct it in one UPDATE.
    // if no row is affected, the user doesn't have enough funds.
    if req_body.side == "SELL" {
        let required_btc = Decimal::from(req_body.qty);

        let result = match sqlx::query(
            "UPDATE balances
             SET balance_btc = balance_btc - $1
             WHERE user_id = $2
               AND balance_btc >= $1",
        )
        .bind(required_btc)
        .bind(claims.id)
        .execute(pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"fail_reason": e.to_string()}));
            }
        };

        if result.rows_affected() == 0 {
            return HttpResponse::InternalServerError().json(serde_json::json!(format!(
                " userId : {} Insufficient Balance :- \n Selling QTY : {}\n",
                claims.id, req_body.qty
            )));
        }
    }

    if req_body.side == "BUY" {
        let required_inr = Decimal::from(req_body.qty) * Decimal::from(req_body.price);

        let result = match sqlx::query(
            "UPDATE balances
             SET balance_inr = balance_inr - $1
             WHERE user_id = $2
               AND balance_inr >= $1",
        )
        .bind(required_inr)
        .bind(claims.id)
        .execute(pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"fail_reason": e.to_string()}));
            }
        };

        if result.rows_affected() == 0 {
            return HttpResponse::InternalServerError().json(serde_json::json!(format!(
                " userId : {} Insufficient Balance :- \n Buying QTY : {}\n",
                claims.id, req_body.qty
            )));
        }
    }

    // call the matching engine
    let order_convert = ConvertToOrder(&req_body, claims.id);

    // lock the orderbook ONLY for the (fast, in-memory) matching step,
    // then release it before the slow database settlement work
    let result = {
        let mut ob = match lock_book(&orderbook) {
            Ok(g) => g,
            Err(resp) => return resp,
        };
        ob.engine(order_convert).await
    };

    // run on_trade for every executed trade
    {
        let mut md = match lock_market(&market_data) {
            Ok(g) => g,
            Err(resp) => return resp,
        };
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
                let mut ob = match lock_book(&orderbook) {
                    Ok(g) => g,
                    Err(resp) => return resp,
                };
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
    let ob = match lock_book(&orderbook) {
        Ok(g) => g,
        Err(resp) => return resp,
    };
    HttpResponse::Ok().json(serde_json::json!({"orderbook": ob.display_orderbook()}))
}
