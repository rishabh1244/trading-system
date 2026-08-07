use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Trade {
    pub buyer_id: i32,
    pub seller_id: i32,

    pub qty: f64,
    pub price: f64,
}

#[derive(Serialize, Deserialize)]
pub struct TradeList {
    pub trades: Vec<Trade>,
}
