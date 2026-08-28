use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub last_price: Decimal,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

pub struct SocketServer {
    pub(crate) tx: broadcast::Sender<String>,
}
