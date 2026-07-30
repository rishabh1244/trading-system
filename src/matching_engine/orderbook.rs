use crate::domain::order::Order;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Mutex;

pub async fn engine(pool: &PgPool, match_data: Order) {
    // Updates the database if the order is not present in the orderbook

    let result = sqlx::query(
        "INSERT INTO orders (user_id, side, qty, price, status)
     VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&match_data.user_id)
    .bind(&match_data.side)
    .bind(&match_data.qty)
    .bind(&match_data.price)
    .bind("pending")
    .execute(pool)
    .await;
}
