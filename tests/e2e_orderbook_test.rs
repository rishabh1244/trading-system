use rust_decimal::Decimal;
use sqlx::PgPool;
use trading_engine::domain::order::Order;
use trading_engine::matching_engine::orderbook::OrderBook;
use trading_engine::trading_engine::engine::settle_trades;
use trading_engine::trading_engine::orderbook::sync_orderbook;

async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

async fn seed_test_data(pool: &PgPool, seller_id: i32, buyer_id: i32) {
    sqlx::query("DELETE FROM trades")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM orders")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM balances")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(seller_id)
        .bind(format!("seller_{}", seller_id))
        .bind("hash")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(buyer_id)
        .bind(format!("buyer_{}", buyer_id))
        .bind("hash")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO balances (user_id, balance_btc, balance_inr, reserved_btc, reserved_inr)
         VALUES ($1, 10, 0, 0, 0)",
    )
    .bind(seller_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO balances (user_id, balance_btc, balance_inr, reserved_btc, reserved_inr)
         VALUES ($1, 0, 10000, 0, 0)",
    )
    .bind(buyer_id)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn buy_order_matches_sell_order_end_to_end() {
    let pool = setup_db().await;
    let seller_id = 1001;
    let buyer_id = 1002;

    // initilise users and there balance
    seed_test_data(&pool, seller_id, buyer_id).await;

    // 1. Insert seller's resting order into DB
    let mut tx = pool.begin().await.unwrap();
    let sell_order = Order {
        order_id: None,
        user_id: seller_id,
        side: "SELL".to_string(),
        qty: Decimal::from(5),
        price: Decimal::from(100),
        status: "pending".to_string(),
    };
    let sell_order_id = sync_orderbook(&mut tx, &sell_order).await.unwrap(); //order into database 
    tx.commit().await.unwrap();

    // 2. Put it into matching engine/orderbook
    let mut orderbook = OrderBook::new();
    let sell_with_id = Order {
        order_id: Some(sell_order_id),
        ..sell_order
    };
    orderbook.add_resting_order(sell_with_id.clone());

    // 3. Submit buyer's order through the actual service
    let buy_order = Order {
        order_id: None,
        user_id: buyer_id,
        side: "BUY".to_string(),
        qty: Decimal::from(5),
        price: Decimal::from(100),
        status: "pending".to_string(),
    };
    let result = orderbook.engine(buy_order.clone()).await;

    // 4. Verify a trade happened
    assert_eq!(result.trades.trades.len(), 1);

    // 5. Settle trades into DB
    let mut tx = pool.begin().await.unwrap();
    let (_updated_balances, new_order_id) = settle_trades(
        buyer_id,
        &buy_order,
        result.trades,
        result.appends,
        result.fulfilled_ids,
        &mut tx,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // 6. Verify DB contains the trade
    let trade: (i32, i32, Decimal, Decimal) =
        sqlx::query_as("SELECT buyer_id, seller_id, qty, price FROM trades LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(trade.0, buyer_id);
    assert_eq!(trade.1, seller_id);
    assert_eq!(trade.2, Decimal::from(5));
    assert_eq!(trade.3, Decimal::from(100));

    // 7. Verify seller order is fulfilled
    let seller_order: (String,) = sqlx::query_as("SELECT status FROM orders WHERE order_id = $1")
        .bind(sell_order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(seller_order.0, "fulfilled");

    // 8. No leftover order
    assert!(new_order_id.is_none());

    // 9. Verify balances: seller gets INR, buyer gets BTC
    let seller_bal: (Decimal, Decimal) =
        sqlx::query_as("SELECT balance_btc, balance_inr FROM balances WHERE user_id = $1")
            .bind(seller_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seller_bal.0, Decimal::from(5)); // 10 - 5 reserved = 5
    assert_eq!(seller_bal.1, Decimal::from(500)); // 0 + 5*100

    let buyer_bal: (Decimal, Decimal) =
        sqlx::query_as("SELECT balance_btc, balance_inr FROM balances WHERE user_id = $1")
            .bind(buyer_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(buyer_bal.0, Decimal::from(5)); // 0 + 5
    assert_eq!(buyer_bal.1, Decimal::from(5000)); // 10000 - 5*100
}

#[tokio::test]
async fn partial_fill_sets_resting_order() {
    let pool = setup_db().await;
    let seller_id = 2001;
    let buyer_id = 2002;

    seed_test_data(&pool, seller_id, buyer_id).await;

    // Seller places resting order for 3 BTC
    let mut tx = pool.begin().await.unwrap();
    let sell_order = Order {
        order_id: None,
        user_id: seller_id,
        side: "SELL".to_string(),
        qty: Decimal::from(3),
        price: Decimal::from(100),
        status: "pending".to_string(),
    };
    let sell_order_id = sync_orderbook(&mut tx, &sell_order).await.unwrap();
    tx.commit().await.unwrap();

    let mut orderbook = OrderBook::new();
    orderbook.add_resting_order(Order {
        order_id: Some(sell_order_id),
        ..sell_order
    });

    // Buyer tries to buy 5 — only 3 available, partial fill
    let buy_order = Order {
        order_id: None,
        user_id: buyer_id,
        side: "BUY".to_string(),
        qty: Decimal::from(5),
        price: Decimal::from(100),
        status: "pending".to_string(),
    };
    let result = orderbook.engine(buy_order.clone()).await;

    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(3));
    assert!(result.appends.is_some());

    let mut tx = pool.begin().await.unwrap();
    let (_updated_balances, new_order_id) = settle_trades(
        buyer_id,
        &buy_order,
        result.trades,
        result.appends,
        result.fulfilled_ids,
        &mut tx,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // A new resting BUY order should be persisted for the remaining 2
    assert!(new_order_id.is_some());

    let resting_order: (String, Decimal, Decimal) =
        sqlx::query_as("SELECT side, qty, price FROM orders WHERE order_id = $1")
            .bind(new_order_id.unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(resting_order.0, "buy");
    assert_eq!(resting_order.1, Decimal::from(2));
    assert_eq!(resting_order.2, Decimal::from(100));

    // Seller's order should be fulfilled
    let seller_order: (String,) = sqlx::query_as("SELECT status FROM orders WHERE order_id = $1")
        .bind(sell_order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(seller_order.0, "fulfilled");
}
