use crate::domain::order::Order;
use crate::domain::trades::{Trade, TradeList};
use sqlx::PgPool;

use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(sqlx::FromRow)]
struct PendingOrderRow {
    order_id: i32,
    user_id: i32,
    side: String,
    qty: i32,
    price: i32,
}

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
/// how do i presist this orderbook ? append_orderbook() should call a db fn .. ig
/// Result of a single pass through the matching engine.
pub struct EngineResult {
    pub trades: TradeList,
    pub appends: Option<Order>,
    pub fulfilled_ids: Vec<i32>,
}

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

    /// Reloads all resting (status = pending) orders from the database
    /// back into the in-memory book. Called at server startup so the
    /// book survives restarts.
    pub async fn load_from_db(&mut self, pool: &PgPool) -> Result<(), sqlx::Error> {
        let rows = sqlx::query_as::<_, PendingOrderRow>(
            "SELECT order_id, user_id, side, qty::int4, price::int4
             FROM orders
             WHERE status = 'pending'
             ORDER BY order_id",
        )
        .fetch_all(pool)
        .await?;

        for row in rows {
            let order = Order {
                order_id: Some(row.order_id),
                user_id: row.user_id,
                side: row.side.to_uppercase(),
                qty: row.qty,
                price: row.price,
                status: "pending".to_string(),
            };
            if order.side == "BUY" {
                self.bids.push(order);
            } else {
                self.asks.push(order);
            }
        }
        Ok(())
    }

    fn orderbook_sell(
        &mut self,
        index: usize,
        match_data: &mut Order,
        trades: &mut TradeList,
        fulfilled_ids: &mut Vec<i32>,
    ) {
        // make the transaction in the orderbook and call the trading engine to db update
        // puchase means removing BTC from asks
        //
        // bids = [100,10]
        // match_data = [120,7]
        let qty = cmp::min(match_data.qty, self.bids[index].qty);

        let price = self.bids[index].price; // price we selling for
        self.bids[index].qty -= qty;
        if self.bids[index].qty == 0 {
            self.bids[index].status = "fulfilled".to_string();
            if let Some(id) = self.bids[index].order_id {
                fulfilled_ids.push(id);
            }
        }

        // TODO (Trading Engine side): update balance of user => balance_prev  - price ;
        match_data.qty -= qty;

        trades.trades.push(Trade {
            buyer_id: self.bids[index].user_id,
            seller_id: match_data.user_id,
            qty: qty as f32,
            price: price as f32,
        });
    }

    fn orderbook_buy(
        &mut self,
        index: usize,
        match_data: &mut Order,
        trades: &mut TradeList,
        fulfilled_ids: &mut Vec<i32>,
    ) {
        // make the transaction in the orderbook and call the trading engine to db update
        // puchase means removing BTC from asks
        //
        //asks = [100,10]
        //match_data = [120,7]
        let qty = cmp::min(match_data.qty, self.asks[index].qty);

        let price = self.asks[index].price; // price we buying for
        self.asks[index].qty -= qty;
        if self.asks[index].qty == 0 {
            self.asks[index].status = "fulfilled".to_string();
            if let Some(id) = self.asks[index].order_id {
                fulfilled_ids.push(id);
            }
        }

        // TODO (Trading Engine side): update balance of user => balance_prev  - price ;
        match_data.qty -= qty;

        trades.trades.push(Trade {
            buyer_id: match_data.user_id,
            seller_id: self.asks[index].user_id,
            qty: qty as f32,
            price: price as f32,
        });
    }

    pub fn display_orderbook(&self) -> serde_json::Value {
        serde_json::json!({
            "asks": self.asks,
            "bids": self.bids,
        })
    }

    pub fn set_last_resting_id(&mut self, side: &str, order_id: i32) {
        match side {
            "BUY" => {
                if let Some(o) = self.bids.last_mut() {
                    o.order_id = Some(order_id);
                }
            }
            "SELL" => {
                if let Some(o) = self.asks.last_mut() {
                    o.order_id = Some(order_id);
                }
            }
            _ => {}
        }
    }

    pub async fn engine(&mut self, mut match_data: Order) -> EngineResult {
        // attempt to match the incoming order against the resting orderbook
        let mut trades = TradeList { trades: Vec::new() };
        let mut appends: Option<Order> = None;
        let mut fulfilled_ids: Vec<i32> = Vec::new();

        if match_data.side == "BUY" {
            // checks asks
            let n = self.asks.len();

            for i in 0..n {
                if self.asks[i].qty <= 0 {
                    continue;
                }
                if match_data.price >= self.asks[i].price {
                    self.orderbook_buy(i, &mut match_data, &mut trades, &mut fulfilled_ids);
                }
            }

            if match_data.qty > 0 {
                // leftover order becomes a resting bid
                self.bids.push(match_data.clone());
                appends = Some(match_data);
            }
        } else if match_data.side == "SELL" {
            let n = self.bids.len();

            for i in 0..n {
                if self.bids[i].qty <= 0 {
                    continue;
                }
                if match_data.price <= self.bids[i].price {
                    self.orderbook_sell(i, &mut match_data, &mut trades, &mut fulfilled_ids);
                }
            }
            if match_data.qty > 0 {
                // leftover order becomes a resting ask
                self.asks.push(match_data.clone());
                appends = Some(match_data);
            }
        }

        EngineResult {
            trades,
            appends,
            fulfilled_ids,
        }
    }
}
