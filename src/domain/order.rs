use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct OrderRequest {
    pub side: String,
    pub qty: i32,
    pub price: i32,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub user_id: i32,
    pub side: String,
    pub qty: i32,
    pub price: i32,
}
