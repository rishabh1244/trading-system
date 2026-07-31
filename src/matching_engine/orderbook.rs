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
pub enum EngineError {
    Database(sqlx::Error),
    InsufficientBalance,
    InvalidOrder,
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

    pub async fn engine(
        &mut self,
        pool: &PgPool,
        match_data: Order,
    ) -> Result<OrderResponse, EngineError> {
        // Updates the database if the order is not present in the orderbook

        // check if the order is present in the orderbook

        let response;
        if match_data.side == "BUY" {
            // checks asks
            let n = self.asks.len();

            for i in 0..n {
                if match_data.price >= self.asks[i].price {
                    match_data.qty = purchase(i, match_data);
                }
            }

            if match_data.qty > 0 {
                response = append_orderbook(match_data);
            }
        } else if match_data.side == "SELL" {
            let n = self.bids.len();

            for i in 0..n {
                if match_data.price <= self.bids[i].price {
                    match_data.qty = sell(i, match_data);
                }
            }
            if match_data.qty > 0 {
                response = append_orderbook(match_data)
            }
            response = "satisfied";
        }

        /*
         *to be done by trading engine
         *
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
        };
        */
        Ok(OrderResponse::Stored {
            order_id: 42,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}
