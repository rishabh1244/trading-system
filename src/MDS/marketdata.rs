use crate::domain::market::MarketData;
use crate::domain::trades::Trade;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::cmp;
/*
 this service maintains
         best bid
         best ask
         last traded price
         recent trades
         order book depth
         candles
*/
impl MarketData {
    pub fn new() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            last_price: 0.0,
        }
    }

    pub fn udpate_service(&mut self, latest_trade: Trade) {
        self.last_price = latestTrade.price;
        self.last_updated = chrono::Utc::now();
    }
}
