use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::domain::trades::Trade;

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub last_price: f64,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub recent_trades: VecDeque<Trade>,
}