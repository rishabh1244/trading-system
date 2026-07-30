use crate::domain::order::Order;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Mutex;

//response to be returned
#[derive(Serialize)]

pub enum OrderResponse {
    Stored { order_id: i32, timestamp: String },
    Approved { order_id: i32, credited: f64 },
    Rejected { reason: String },
}
pub struct OrderElement {
    qty: i32,
    price: i32,
    side: i32,
    order_id: i32,
}

//static ORDER_BOOK: LazyLock<Mutex<Vec<OrderElement>>> =    LazyLock::new(|| Mutex::new(Vec::new()));

pub struct OrderBook {
    bids: Vec<Order>, // Buy orders
    asks: Vec<Order>, // Sell orders
}
impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    pub async fn engine(&mut self, pool: &PgPool, match_data: Order) -> Result<OrderResponse, Err> {
        // Updates the database if the order is not present in the orderbook

        // check if the order is present in the orderbooka

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
}
