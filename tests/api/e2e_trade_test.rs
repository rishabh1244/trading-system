use actix_cors::Cors;
use actix_web::{http::header, test, web, App};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::{Arc, Mutex, OnceLock};
use trading_engine::auth;
use trading_engine::domain::common::{AuthResponseSuccess, Balances};
use trading_engine::domain::market::{MarketData, SocketServer};
use trading_engine::matching_engine::orderbook::OrderBook;
use trading_engine::middleware::auth_middleware::validator;
use trading_engine::OMS::order_management::{
    display_orderbook, fetch_order, get_balance, get_my_orders,
};
use actix_web_httpauth::middleware::HttpAuthentication;

static DB_POOL: OnceLock<PgPool> = OnceLock::new();

fn ensure_env() {
    if std::env::var("JWT_SECRET").is_err() {
        // SAFETY: tests run single-threaded per test binary
        unsafe { std::env::set_var("JWT_SECRET", "test_secret_for_api_tests") };
    }
}

fn get_pool() -> &'static PgPool {
    ensure_env();
    DB_POOL.get_or_init(|| {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost:5432/trading_engine".to_string());
        sqlx::PgPool::connect_lazy(&database_url).expect("failed to connect to database")
    })
}

async fn cleanup_user(pool: &PgPool, username: &str) {
    if let Ok(Some((uid,))) = sqlx::query_as::<_, (i32,)>("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
    {
        let _ = sqlx::query("DELETE FROM trades WHERE buyer_id = $1 OR seller_id = $1")
            .bind(uid).execute(pool).await;
        let _ = sqlx::query("DELETE FROM orders WHERE user_id = $1")
            .bind(uid).execute(pool).await;
        let _ = sqlx::query("DELETE FROM balances WHERE user_id = $1")
            .bind(uid).execute(pool).await;
    }
    let _ = sqlx::query("DELETE FROM users WHERE username = $1")
        .bind(username).execute(pool).await;
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
// ============================================================
#[actix_web::test]
async fn buy_order_matches_sell_order_end_to_end_via_api() {
    let pool = get_pool();
    let seller_user = "e2e_seller_1";
    let buyer_user = "e2e_buyer_1";

    cleanup_user(pool, seller_user).await;
    cleanup_user(pool, buyer_user).await;

    let app = test::init_service(build_app!(pool)).await;

    let seller_token = register_user!(&app, seller_user, "pass123");
    let buyer_token = register_user!(&app, buyer_user, "pass456");

    set_balance(pool, seller_user, 10, 0).await;
    set_balance(pool, buyer_user, 0, 10000).await;

    let resp = auth_post!(&app, seller_token, "/api/order", serde_json::json!({"side": "SELL", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 200, "seller order failed");

    let resp = auth_post!(&app, buyer_token, "/api/order", serde_json::json!({"side": "BUY", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 200, "buyer order failed");
    let buyer_bal: Balances = test::read_body_json(resp).await;
    assert_eq!(buyer_bal.balance_btc, Decimal::from(5));
    assert_eq!(buyer_bal.balance_inr, Decimal::from(9500));

    let resp = auth_get!(&app, seller_token, "/api/balance");
    assert_eq!(resp.status(), 200);
    let seller_bal: Balances = test::read_body_json(resp).await;
    assert_eq!(seller_bal.balance_btc, Decimal::from(5));
    assert_eq!(seller_bal.balance_inr, Decimal::from(500));

    let trade: (Decimal, Decimal) = sqlx::query_as(
        "SELECT qty, price FROM trades
         WHERE buyer_id = (SELECT id FROM users WHERE username = $1)
           AND seller_id = (SELECT id FROM users WHERE username = $2)",
    )
    .bind(buyer_user)
    .bind(seller_user)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(trade.0, Decimal::from(5));
    assert_eq!(trade.1, Decimal::from(100));

    let order_status: (String,) = sqlx::query_as(
        "SELECT status FROM orders
         WHERE user_id = (SELECT id FROM users WHERE username = $1)
           AND upper(side) = 'SELL'",
    )
    .bind(seller_user)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(order_status.0, "fulfilled");

    cleanup_user(pool, seller_user).await;
    cleanup_user(pool, buyer_user).await;
}

// ============================================================
// Test 2: Partial fill creates resting order via API
// ============================================================
#[actix_web::test]
async fn partial_fill_sets_resting_order_via_api() {
    let pool = get_pool();
    let seller_user = "e2e_seller_2";
    let buyer_user = "e2e_buyer_2";

    cleanup_user(pool, seller_user).await;
    cleanup_user(pool, buyer_user).await;

    let app = test::init_service(build_app!(pool)).await;

    let seller_token = register_user!(&app, seller_user, "pass123");
    let buyer_token = register_user!(&app, buyer_user, "pass456");

    set_balance(pool, seller_user, 10, 0).await;
    set_balance(pool, buyer_user, 0, 10000).await;

    let resp = auth_post!(&app, seller_token, "/api/order", serde_json::json!({"side": "SELL", "qty": 3, "price": 100}));
    assert_eq!(resp.status(), 200);

    let resp = auth_post!(&app, buyer_token, "/api/order", serde_json::json!({"side": "BUY", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 200);
    let buyer_bal: Balances = test::read_body_json(resp).await;
    assert_eq!(buyer_bal.balance_btc, Decimal::from(3));
    assert_eq!(buyer_bal.balance_inr, Decimal::from(9700));

    let resp = auth_get!(&app, seller_token, "/api/balance");
    assert_eq!(resp.status(), 200);
    let seller_bal: Balances = test::read_body_json(resp).await;
    assert_eq!(seller_bal.balance_btc, Decimal::from(7));
    assert_eq!(seller_bal.balance_inr, Decimal::from(300));

    let buyer_id: (i32,) = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(buyer_user)
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

    let order_status: (String,) = sqlx::query_as(
        "SELECT status FROM orders
         WHERE user_id = (SELECT id FROM users WHERE username = $1)
           AND upper(side) = 'SELL'",
    )
    .bind(seller_user)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(order_status.0, "fulfilled");

    cleanup_user(pool, seller_user).await;
    cleanup_user(pool, buyer_user).await;
}

// ============================================================
// Test 3: Insufficient BTC is rejected
// ============================================================
#[actix_web::test]
async fn sell_order_rejected_on_insufficient_btc() {
    let pool = get_pool();
    let seller_user = "e2e_seller_ins";

    cleanup_user(pool, seller_user).await;

    let app = test::init_service(build_app!(pool)).await;

    let token = register_user!(&app, seller_user, "pass123");
    set_balance(pool, seller_user, 2, 0).await;

    let resp = auth_post!(&app, token, "/api/order", serde_json::json!({"side": "SELL", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 500, "should reject insufficient BTC");

    cleanup_user(pool, seller_user).await;
}

// ============================================================
// Test 4: Insufficient INR is rejected
// ============================================================
#[actix_web::test]
async fn buy_order_rejected_on_insufficient_inr() {
    let pool = get_pool();
    let buyer_user = "e2e_buyer_ins";

    cleanup_user(pool, buyer_user).await;

    let app = test::init_service(build_app!(pool)).await;

    let token = register_user!(&app, buyer_user, "pass456");
    set_balance(pool, buyer_user, 0, 100).await;

    let resp = auth_post!(&app, token, "/api/order", serde_json::json!({"side": "BUY", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 500, "should reject insufficient INR");

    cleanup_user(pool, buyer_user).await;
}

// ============================================================
// Test 5: Order without auth is rejected
// ============================================================
#[actix_web::test]
async fn order_without_auth_is_rejected() {
    let pool = get_pool();
    let app = test::init_service(build_app!(pool)).await;

    let req = test::TestRequest::post()
        .uri("/api/order")
        .set_json(serde_json::json!({"side": "SELL", "qty": 5, "price": 100}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "should reject unauthenticated request");
}

// ============================================================
// Test 6: Balance and my-orders reflect completed trade
// ============================================================
#[actix_web::test]
async fn balance_and_orders_reflect_trade() {
    let pool = get_pool();
    let seller_user = "e2e_seller_bo";
    let buyer_user = "e2e_buyer_bo";

    cleanup_user(pool, seller_user).await;
    cleanup_user(pool, buyer_user).await;

    let app = test::init_service(build_app!(pool)).await;

    let seller_token = register_user!(&app, seller_user, "pass123");
    let buyer_token = register_user!(&app, buyer_user, "pass456");

    set_balance(pool, seller_user, 10, 0).await;
    set_balance(pool, buyer_user, 0, 10000).await;

    let resp = auth_post!(&app, seller_token, "/api/order", serde_json::json!({"side": "SELL", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 200, "seller order failed");

    let resp = auth_post!(&app, buyer_token, "/api/order", serde_json::json!({"side": "BUY", "qty": 5, "price": 100}));
    assert_eq!(resp.status(), 200, "buyer order failed");

    let resp = auth_get!(&app, seller_token, "/api/my-orders");
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert_eq!(status, 200, "seller /api/my-orders failed: {}", String::from_utf8_lossy(&body_bytes));
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let orders = body.as_array().expect("orders should be an array");
    assert!(!orders.is_empty(), "seller should have at least one order");
    assert_eq!(orders[0]["status"], "fulfilled");

    let resp = auth_get!(&app, buyer_token, "/api/my-orders");
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    assert_eq!(status, 200, "buyer /api/my-orders failed: {}", String::from_utf8_lossy(&body_bytes));
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let orders = body.as_array().expect("orders should be an array");
    assert!(!orders.is_empty(), "buyer should have at least one order");

    cleanup_user(pool, seller_user).await;
    cleanup_user(pool, buyer_user).await;
}
