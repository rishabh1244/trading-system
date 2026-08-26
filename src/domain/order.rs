use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct OrderRequest {
    pub side: String,
    pub qty: i32,
    pub price: i32,
}

#[derive(sqlx::FromRow)]
pub struct PendingOrderRow {
    pub order_id: i32,
    pub user_id: i32,
    pub side: String,
    pub qty: i32,
    pub price: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Order {
    pub order_id: Option<i32>,
    pub user_id: i32,
    pub side: String,
    pub qty: i32,
    pub price: i32,
    pub status: String,
}
