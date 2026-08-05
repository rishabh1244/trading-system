use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Trade {
    pub buyer_id: i32,
    pub seller_id: i32,

    pub qty: f32,
    pub price: f32,
}

#[derive(Serialize, Deserialize)]
pub struct TradeList {
    pub trades: Vec<Trade>,
}
