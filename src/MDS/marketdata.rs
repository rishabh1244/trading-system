use crate::domain::market::MarketData;
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
impl MarketData {
    pub fn new() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            last_price: rust_decimal::Decimal::ZERO,
        }
    }

    pub fn on_trade(&mut self, trade: &Trade) {
        self.last_price = trade.price;
        self.last_updated = chrono::Utc::now();
    }
}
