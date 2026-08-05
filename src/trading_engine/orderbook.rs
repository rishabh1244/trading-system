use crate::domain::order::Order;
use crate::trading_engine::engine::update_balance;
use sqlx::PgPool;

/// Persists the resting (unfilled) orders to the database.
/// For each order it:
/// - debits the appropriate balance (SELL -> BTC, BUY -> INR) so funds are locked
/// - inserts the order into the `orders` table with status `pending`
pub async fn sync_orderbook(pool: &PgPool, appends: &[Order]) -> Result<(), sqlx::Error> {
    for order in appends {
        let _ = update_balance(order.user_id, &order.side, order.qty, order.price, pool).await?;

        sqlx::query(
            "INSERT INTO orders (user_id, side, qty, price, status)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(order.user_id)
        .bind(order.side.to_lowercase())
        .bind(order.qty)
        .bind(order.price)
        .bind("pending")
        .execute(pool)
        .await?;
    }
    Ok(())
}