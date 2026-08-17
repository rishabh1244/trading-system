use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub last_price: Decimal,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

pub struct SocketServer {
    pub(crate) clients: Arc<Mutex<Vec<TcpStream>>>,
}
