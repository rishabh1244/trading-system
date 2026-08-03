use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Trades {
    pub trade_id: i32,
    pub trader_id: i32,
    pub seller_id: i32,

    pub qty: f32,
    pub price: f32,
}
