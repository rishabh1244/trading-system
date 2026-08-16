use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::domain::trades::Trade;

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub last_price: Decimal,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}
