use crate::domain::order::Order;
use crate::trading_engine::engine::update_balance;
use sqlx::{PgPool, Row};

/// Persists the resting (unfilled) order to the database.
/// It:
/// - debits the appropriate balance (SELL -> BTC, BUY -> INR) so funds are locked
/// - inserts the order into the `orders` table with status `pending`
/// Returns the generated order_id.
pub async fn sync_orderbook(pool: &PgPool, order: &Order) -> Result<i32, sqlx::Error> {
    let _ = update_balance(order.user_id, &order.side, order.qty, order.price, pool).await?;

    let row = sqlx::query(
        "INSERT INTO orders (user_id, side, qty, price, status)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING order_id",
    )
    .bind(order.user_id)
    .bind(order.side.to_lowercase())
    .bind(order.qty)
    .bind(order.price)
    .bind("pending")
    .fetch_one(pool)
    .await?;

    Ok(row.get("order_id"))
}

