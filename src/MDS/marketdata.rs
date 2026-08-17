use crate::domain::market::{MarketData, SocketServer};
use crate::domain::trades::Trade;

/*
 this service maintains
         best bid
         best ask
         last traded price
         recent trades
         order book depth
         candles
*/

// trading engine service should keep on updating this data
// and then socket server should broadcast .

impl MarketData {
    pub fn new() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            last_price: rust_decimal::Decimal::ZERO,
        }
    }

    pub fn on_trade(&mut self, trade: &Trade, server: &SocketServer) {
        self.last_price = trade.price;
        self.last_updated = chrono::Utc::now();

        let json = serde_json::to_string(self).unwrap();
        server.broadcast(&json);
    }
}
