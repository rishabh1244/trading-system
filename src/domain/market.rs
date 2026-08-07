use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::domain::trades::Trade;

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub last_price: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}
