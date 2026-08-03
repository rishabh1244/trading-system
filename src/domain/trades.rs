use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Trade {
    pub trade_id: i32,
    pub trader_id: i32,
    pub seller_id: i32,

    pub qty: f32,
    pub price: f32,
}

#[derive(Deserialize)]
pub struct TradeList {
    pub trades: Vec<Trade>,
}
