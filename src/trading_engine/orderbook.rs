use crate::domain::order::Order;
use sqlx::{Postgres, Row, Transaction};

/// Persists the resting (unfilled) order to the database.
/// Funds were already locked atomically when the order was placed,
/// so this only inserts the order with status `pending`.
/// Returns the generated order_id.
pub async fn sync_orderbook(
    tx: &mut Transaction<'_, Postgres>,
    order: &Order,
) -> Result<i32, sqlx::Error> {
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
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.get("order_id"))
}
