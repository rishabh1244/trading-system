use crate::domain::order::Order;
use crate::domain::trades::Trade;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::cmp;
use std::sync::Mutex;
use std::task::Wake;

//response to be returned
#[derive(Serialize, Debug)]

pub enum OrderResponse {
    Stored { order_id: i32, timestamp: String },
    Approved { order_id: i32, credited: f64 },
    Rejected { reason: String },
}
#[derive(Debug)]
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
    bids: Vec<Order>, // Limit BUY orders (people waiting to buy)
    asks: Vec<Order>, // Limit SELL orders (people waiting to orderbook_sell)
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    fn orderbook_sell(&mut self, index: usize, mut match_data: &Order) -> i32 {
        // make the transaction in the orderbook and call the trading engine to db update
        // puchase means removing BTC from asks
        //
        // bids = [100,10]
        // match_data = [120,7]
        let qty = cmp::min(match_data.qty, self.bids[index].qty);

        let price = self.bids[index].price * qty; // price we buying for 
        self.bids[index].qty -= qty;

        // TODO (Trading Engine side): update balance of user => balance_prev  - price ;

        return qty;
    }

    fn orderbook_buy(&mut self, index: usize, match_data: &Order) -> i32 {
        // make the transaction in the orderbook and call the trading engine to db update
        // puchase means removing BTC from asks
        //
        //asks = [100,10]
        //match_data = [120,7]
        let qty = cmp::min(match_data.qty, self.asks[index].qty);

        let price = self.asks[index].price * qty; // price we buying for 
        self.asks[index].qty -= qty;

        // TODO (Trading Engine side): update balance of user => balance_prev  - price ;

        return qty;
    }

    fn append_orderbook(&mut self, order_type: &str, match_data: &Order) {
        // appends the unfinished order to the orderbook
        // udpates the database after that
        // TODO -> further optimise the orderbook to stay sorted for easy searching

        // currently just adding a dumb append
        if order_type == "ASK".to_string() {
            self.asks.push(match_data.clone());
        }
        if order_type == "BID".to_string() {
            self.bids.push(match_data.clone());
        }
    }

    pub fn display_orderbook(&self) -> serde_json::Value {
        serde_json::json!({
            "asks": self.asks,
            "bids": self.bids,
        })
    }

    pub async fn engine(&mut self, pool: &PgPool, mut match_data: Order) -> Trade {
        // Updates the database if the order is not present in the orderbook

        // check if the order is present in the orderbook

        if match_data.side == "BUY" {
            // checks asks
            let n = self.asks.len();

            for i in 0..n {
                if match_data.price >= self.asks[i].price {
                    match_data.qty -= self.orderbook_buy(i, &match_data);
                }
            }

            if match_data.qty > 0 {
                self.append_orderbook("BID", &match_data);
            }
        } else if match_data.side == "SELL" {
            let n = self.bids.len();

            for i in 0..n {
                if match_data.price <= self.bids[i].price {
                    match_data.qty -= self.orderbook_sell(i, &match_data);
                }
            }
            if match_data.qty > 0 {
                self.append_orderbook("ASK", &match_data)
            }
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
        // should return a Trade type
        Ok(OrderResponse::Stored {
            order_id: 42,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}
