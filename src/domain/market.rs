use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub last_price: Decimal,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}
