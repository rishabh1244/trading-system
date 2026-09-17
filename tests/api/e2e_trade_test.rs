use actix_cors::Cors;
use actix_web::{App, http::header, test, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use trading_engine::OMS::order_management::{
    display_orderbook, fetch_order, get_balance, get_my_orders,
};
use trading_engine::auth;
use trading_engine::domain::common::{AuthResponseSuccess, Balances};
use trading_engine::domain::market::{MarketData, SocketServer};
use trading_engine::matching_engine::orderbook::OrderBook;
use trading_engine::middleware::auth_middleware::validator;

fn total_btc(b: &Balances) -> Decimal {
    b.balance_btc + b.reserved_btc
}

fn total_inr(b: &Balances) -> Decimal {
    b.balance_inr + b.reserved_inr
}

/// Delete a single test user and all their child rows (trades, orders,
/// balances).  Only touches the one user — never does a global delete.
async fn cleanup_user(pool: &PgPool, username: &str) {
    if let Ok(Some((uid,))) =
        sqlx::query_as::<_, (i32,)>("SELECT id FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await
    {
        sqlx::query("DELETE FROM trades WHERE buyer_id = $1 OR seller_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM orders WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM balances WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn set_balance(pool: &PgPool, username: &str, btc: i32, inr: i32) {
    let (uid,): (i32,) = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE balances SET balance_btc = $1, balance_inr = $2, reserved_btc = 0, reserved_inr = 0 WHERE user_id = $3",
    )
    .bind(Decimal::from(btc))
    .bind(Decimal::from(inr))
    .bind(uid)
    .execute(pool)
    .await
    .unwrap();
}

async fn get_total_assets(pool: &PgPool, username: &str) -> (Decimal, Decimal) {
    let row: (Decimal, Decimal, Decimal, Decimal) = sqlx::query_as(
        "SELECT balance_btc, balance_inr, reserved_btc, reserved_inr FROM balances WHERE user_id = (SELECT id FROM users WHERE username = $1)",
    )
    .bind(username)
    .fetch_one(pool)
    .await
    .unwrap();
    (row.0 + row.2, row.1 + row.3)
}

macro_rules! build_app {
    ($pool:expr) => {{
        let auth_mw = HttpAuthentication::bearer(validator);
        App::new()
            .wrap(Cors::permissive())
            .app_data(web::Data::new(Some($pool.clone())))
            .app_data(web::Data::new(Arc::new(Mutex::new(OrderBook::new()))))
            .app_data(web::Data::new(Arc::new(Mutex::new(MarketData::new()))))
            .app_data(web::Data::new(Arc::new(SocketServer::new())))
            .service(auth::register::register_user)
            .service(auth::login::login_user)
            .service(
                web::scope("")
                    .wrap(auth_mw)
                    .service(fetch_order)
                    .service(display_orderbook)
                    .service(get_balance)
                    .service(get_my_orders),
            )
    }};
}

macro_rules! register_user {
    ($app:expr, $username:expr, $password:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/register")
            .set_json(serde_json::json!({"username": $username, "password": $password}))
            .to_request();
        let resp = test::call_service($app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        assert_eq!(status, 200, "register {} failed: {}", $username, String::from_utf8_lossy(&body));
        let parsed: AuthResponseSuccess = serde_json::from_slice(&body).unwrap();
        parsed.token
    }};
}

macro_rules! auth_post {
    ($app:expr, $token:expr, $uri:expr, $json:expr) => {{
        let req = test::TestRequest::post()
            .uri($uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", $token)))
            .set_json($json)
            .to_request();
        test::call_service($app, req).await
    }};
}

macro_rules! auth_get {
    ($app:expr, $token:expr, $uri:expr) => {{
        let req = test::TestRequest::get()
            .uri($uri)
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", $token)))
            .to_request();
        test::call_service($app, req).await
    }};
}

// ============================================================
// Test 1: Full buy-matches-sell end-to-end via API
//
// Asset flow (total = balance + reserved):
//   Seller: 10 BTC, 0 INR  →  5 BTC, 500 INR  (sold 5 BTC @ 100)
//   Buyer:  0 BTC, 10000 INR  →  5 BTC, 9500 INR  (bought 5 BTC @ 100)
// ============================================================
#[actix_web::test]
async fn buy_order_matches_sell_order_end_to_end_via_api() {
    let pool = super::get_pool();
    let seller_user = super::unique_user("e2e_seller1");
    let buyer_user = super::unique_user("e2e_buyer1");

    let app = test::init_service(build_app!(pool)).await;

    cleanup_user(pool, &seller_user).await;
    cleanup_user(pool, &buyer_user).await;
    let seller_token = register_user!(&app, seller_user, "pass123");
    let buyer_token = register_user!(&app, buyer_user, "pass456");

    set_balance(pool, &seller_user, 10, 0).await;
    set_balance(pool, &buyer_user, 0, 10000).await;

    // seller places SELL 5 @ 100 — reserves 5 BTC
    let resp = auth_post!(
        &app,
        seller_token,
        "/api/order",
        serde_json::json!({"side": "SELL", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 200, "seller order failed");

    // buyer places BUY 5 @ 100 — reserves 500 INR, trade matches fully
    let resp = auth_post!(
        &app,
        buyer_token,
        "/api/order",
        serde_json::json!({"side": "BUY", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 200, "buyer order failed");
    let buyer_bal: Balances = test::read_body_json(resp).await;

    // buyer got 5 BTC, spent 500 INR; all reserved funds consumed on full fill
    assert_eq!(total_btc(&buyer_bal), Decimal::from(5), "buyer total BTC");
    assert_eq!(total_inr(&buyer_bal), Decimal::from(9500), "buyer total INR");
    assert_eq!(buyer_bal.reserved_btc, Decimal::ZERO, "buyer reserved BTC should be 0 after full fill");
    assert_eq!(buyer_bal.reserved_inr, Decimal::ZERO, "buyer reserved INR should be 0 after full fill");

    // seller side
    let resp = auth_get!(&app, seller_token, "/api/balance");
    assert_eq!(resp.status(), 200);
    let seller_bal: Balances = test::read_body_json(resp).await;
    assert_eq!(total_btc(&seller_bal), Decimal::from(5), "seller total BTC");
    assert_eq!(total_inr(&seller_bal), Decimal::from(500), "seller total INR");
    assert_eq!(seller_bal.reserved_btc, Decimal::ZERO, "seller reserved BTC should be 0 after full fill");
    assert_eq!(seller_bal.reserved_inr, Decimal::ZERO, "seller reserved INR should be 0 after full fill");

    // cross-check via DB
    let (seller_total_btc, seller_total_inr) = get_total_assets(pool, &seller_user).await;
    assert_eq!(seller_total_btc, Decimal::from(5));
    assert_eq!(seller_total_inr, Decimal::from(500));
    let (buyer_total_btc, buyer_total_inr) = get_total_assets(pool, &buyer_user).await;
    assert_eq!(buyer_total_btc, Decimal::from(5));
    assert_eq!(buyer_total_inr, Decimal::from(9500));

    // trade record in DB
    let trade: (Decimal, Decimal) = sqlx::query_as(
        "SELECT qty, price FROM trades
         WHERE buyer_id = (SELECT id FROM users WHERE username = $1)
           AND seller_id = (SELECT id FROM users WHERE username = $2)",
    )
    .bind(&buyer_user)
    .bind(&seller_user)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(trade.0, Decimal::from(5));
    assert_eq!(trade.1, Decimal::from(100));

    // seller order fulfilled
    let order_status: (String,) = sqlx::query_as(
        "SELECT status FROM orders
         WHERE user_id = (SELECT id FROM users WHERE username = $1)
           AND upper(side) = 'SELL'",
    )
    .bind(&seller_user)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(order_status.0, "fulfilled");

    cleanup_user(pool, &seller_user).await;
    cleanup_user(pool, &buyer_user).await;
}

// ============================================================
// Test 2: Partial fill creates resting order via API
//
// Seller SELL 3 @ 100, Buyer BUY 5 @ 100 → 3 matched, 2 resting
//
// Asset flow (total = balance + reserved):
//   Seller: 10 BTC, 0 INR  →  7 BTC, 300 INR  (sold 3 BTC @ 100)
//   Buyer:  0 BTC, 10000 INR  →  3 BTC, 9700 INR  (bought 3 BTC @ 100,
//           200 INR reserved for resting buy order of 2 BTC @ 100)
// ============================================================
#[actix_web::test]
async fn partial_fill_sets_resting_order_via_api() {
    let pool = super::get_pool();
    let seller_user = super::unique_user("e2e_seller2");
    let buyer_user = super::unique_user("e2e_buyer2");

    let app = test::init_service(build_app!(pool)).await;

    cleanup_user(pool, &seller_user).await;
    cleanup_user(pool, &buyer_user).await;
    let seller_token = register_user!(&app, seller_user, "pass123");
    let buyer_token = register_user!(&app, buyer_user, "pass456");

    set_balance(pool, &seller_user, 10, 0).await;
    set_balance(pool, &buyer_user, 0, 10000).await;

    // seller places SELL 3 @ 100 — reserves 3 BTC
    let resp = auth_post!(
        &app,
        seller_token,
        "/api/order",
        serde_json::json!({"side": "SELL", "qty": 3, "price": 100})
    );
    assert_eq!(resp.status(), 200, "seller order failed");

    // buyer places BUY 5 @ 100 — reserves 500 INR, only 3 matched
    let resp = auth_post!(
        &app,
        buyer_token,
        "/api/order",
        serde_json::json!({"side": "BUY", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 200, "buyer order failed");

    let buyer_bal: Balances = test::read_body_json(resp).await;
    // buyer total: 3 BTC received, 300 INR spent; 200 INR still reserved for resting order
    assert_eq!(total_btc(&buyer_bal), Decimal::from(3), "buyer total BTC");
    assert_eq!(total_inr(&buyer_bal), Decimal::from(9700), "buyer total INR (10000 - 300 spent)");
    assert_eq!(buyer_bal.reserved_inr, Decimal::from(200), "buyer reserved INR for resting order");

    // seller side
    let resp = auth_get!(&app, seller_token, "/api/balance");
    assert_eq!(resp.status(), 200);
    let seller_bal: Balances = test::read_body_json(resp).await;
    assert_eq!(total_btc(&seller_bal), Decimal::from(7), "seller total BTC");
    assert_eq!(total_inr(&seller_bal), Decimal::from(300), "seller total INR");
    assert_eq!(seller_bal.reserved_btc, Decimal::ZERO, "seller reserved BTC fully consumed");

    // cross-check via DB
    let (seller_total_btc, seller_total_inr) = get_total_assets(pool, &seller_user).await;
    assert_eq!(seller_total_btc, Decimal::from(7));
    assert_eq!(seller_total_inr, Decimal::from(300));
    let (buyer_total_btc, buyer_total_inr) = get_total_assets(pool, &buyer_user).await;
    assert_eq!(buyer_total_btc, Decimal::from(3));
    assert_eq!(buyer_total_inr, Decimal::from(9700));

    // resting order for the unfilled 2 BTC
    let buyer_id: (i32,) = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(&buyer_user)
        .fetch_one(pool)
        .await
        .unwrap();
    let resting: (String, Decimal, Decimal) = sqlx::query_as(
        "SELECT side, qty, price FROM orders WHERE user_id = $1 AND status = 'pending' ORDER BY order_id DESC LIMIT 1",
    )
    .bind(buyer_id.0)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(resting.0.to_lowercase(), "buy");
    assert_eq!(resting.1, Decimal::from(2));
    assert_eq!(resting.2, Decimal::from(100));

    // seller order fulfilled
    let order_status: (String,) = sqlx::query_as(
        "SELECT status FROM orders
         WHERE user_id = (SELECT id FROM users WHERE username = $1)
           AND upper(side) = 'SELL'",
    )
    .bind(&seller_user)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(order_status.0, "fulfilled");

    cleanup_user(pool, &seller_user).await;
    cleanup_user(pool, &buyer_user).await;
}

// ============================================================
// Test 3: Insufficient BTC is rejected
// ============================================================
#[actix_web::test]
async fn sell_order_rejected_on_insufficient_btc() {
    let pool = super::get_pool();
    let seller_user = super::unique_user("e2e_ins_btc");

    let app = test::init_service(build_app!(pool)).await;

    cleanup_user(pool, &seller_user).await;
    let token = register_user!(&app, seller_user, "pass123");
    set_balance(pool, &seller_user, 2, 0).await;

    let resp = auth_post!(
        &app,
        token,
        "/api/order",
        serde_json::json!({"side": "SELL", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 500, "should reject insufficient BTC");

    // total assets unchanged
    let (total_b, total_i) = get_total_assets(pool, &seller_user).await;
    assert_eq!(total_b, Decimal::from(2), "BTC unchanged after rejected order");
    assert_eq!(total_i, Decimal::ZERO, "INR unchanged after rejected order");

    cleanup_user(pool, &seller_user).await;
}

// ============================================================
// Test 4: Insufficient INR is rejected
// ============================================================
#[actix_web::test]
async fn buy_order_rejected_on_insufficient_inr() {
    let pool = super::get_pool();
    let buyer_user = super::unique_user("e2e_ins_inr");

    let app = test::init_service(build_app!(pool)).await;

    cleanup_user(pool, &buyer_user).await;
    let token = register_user!(&app, buyer_user, "pass456");
    set_balance(pool, &buyer_user, 0, 100).await;

    let resp = auth_post!(
        &app,
        token,
        "/api/order",
        serde_json::json!({"side": "BUY", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 500, "should reject insufficient INR");

    // total assets unchanged
    let (total_b, total_i) = get_total_assets(pool, &buyer_user).await;
    assert_eq!(total_b, Decimal::ZERO, "BTC unchanged after rejected order");
    assert_eq!(total_i, Decimal::from(100), "INR unchanged after rejected order");

    cleanup_user(pool, &buyer_user).await;
}

// ============================================================
// Test 5: Order without auth is rejected
// ============================================================
#[actix_web::test]
async fn order_without_auth_is_rejected() {
    let pool = super::get_pool();
    let app = test::init_service(build_app!(pool)).await;

    let req = test::TestRequest::post()
        .uri("/api/order")
        .set_json(serde_json::json!({"side": "SELL", "qty": 5, "price": 100}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should reject unauthenticated request");
}

// ============================================================
// Test 6: Balance and my-orders reflect completed trade via API
//
// Same setup as test 1 but also checks /api/my-orders endpoint.
// Asset flow (total = balance + reserved):
//   Seller: 10 BTC, 0 INR  →  5 BTC, 500 INR
//   Buyer:  0 BTC, 10000 INR  →  5 BTC, 9500 INR
// ============================================================
#[actix_web::test]
async fn balance_and_orders_reflect_trade() {
    let pool = super::get_pool();
    let seller_user = super::unique_user("e2e_seller_bo");
    let buyer_user = super::unique_user("e2e_buyer_bo");

    let app = test::init_service(build_app!(pool)).await;

    cleanup_user(pool, &seller_user).await;
    cleanup_user(pool, &buyer_user).await;
    let seller_token = register_user!(&app, seller_user, "pass123");
    let buyer_token = register_user!(&app, buyer_user, "pass456");

    set_balance(pool, &seller_user, 10, 0).await;
    set_balance(pool, &buyer_user, 0, 10000).await;

    let resp = auth_post!(
        &app,
        seller_token,
        "/api/order",
        serde_json::json!({"side": "SELL", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 200, "seller order failed");

    let resp = auth_post!(
        &app,
        buyer_token,
        "/api/order",
        serde_json::json!({"side": "BUY", "qty": 5, "price": 100})
    );
    assert_eq!(resp.status(), 200, "buyer order failed");

    // verify total assets via API
    let resp = auth_get!(&app, seller_token, "/api/balance");
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert_eq!(
        status,
        200,
        "seller /api/balance failed: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    let seller_bal: Balances = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(total_btc(&seller_bal), Decimal::from(5), "seller total BTC");
    assert_eq!(total_inr(&seller_bal), Decimal::from(500), "seller total INR");

    let resp = auth_get!(&app, buyer_token, "/api/balance");
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert_eq!(
        status,
        200,
        "buyer /api/balance failed: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    let buyer_bal: Balances = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(total_btc(&buyer_bal), Decimal::from(5), "buyer total BTC");
    assert_eq!(total_inr(&buyer_bal), Decimal::from(9500), "buyer total INR");

    // verify orders endpoint — seller's resting order is persisted as fulfilled
    let resp = auth_get!(&app, seller_token, "/api/my-orders");
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert_eq!(
        status,
        200,
        "seller /api/my-orders failed: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let orders = body.as_array().expect("orders should be an array");
    assert!(!orders.is_empty(), "seller should have at least one order");
    assert_eq!(orders[0]["status"], "fulfilled");

    // buyer's incoming order was fully matched and consumed — it is not
    // persisted to the orders table (only resting orders are), so
    // /api/my-orders returns an empty list. This is current behaviour.
    let resp = auth_get!(&app, buyer_token, "/api/my-orders");
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert_eq!(
        status,
        200,
        "buyer /api/my-orders failed: {}",
        String::from_utf8_lossy(&body_bytes)
    );
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let orders = body.as_array().expect("orders should be an array");
    assert!(
        orders.is_empty(),
        "buyer fully-matched order should not be persisted (only resting orders are)"
    );

    cleanup_user(pool, &seller_user).await;
    cleanup_user(pool, &buyer_user).await;
}
