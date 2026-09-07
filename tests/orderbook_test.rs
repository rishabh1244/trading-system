use rust_decimal::Decimal;
use trading_engine::domain::order::Order;
use trading_engine::matching_engine::orderbook::OrderBook;

fn make_order(user_id: i32, side: &str, qty: i32, price: i32) -> Order {
    Order {
        order_id: None,
        user_id,
        side: side.to_string(),
        qty: Decimal::from(qty),
        price: Decimal::from(price),
        status: "pending".to_string(),
    }
}

fn make_order_with_id(order_id: i32, user_id: i32, side: &str, qty: i32, price: i32) -> Order {
    Order {
        order_id: Some(order_id),
        user_id,
        side: side.to_string(),
        qty: Decimal::from(qty),
        price: Decimal::from(price),
        status: "pending".to_string(),
    }
}

#[test]
fn new_orderbook_is_empty() {
    let ob = OrderBook::new();
    let json = ob.display_orderbook();
    assert_eq!(json["asks"], serde_json::json!({}));
    assert_eq!(json["bids"], serde_json::json!({}));
}

#[test]
fn add_resting_buy_order() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 10, 100));
    let json = ob.display_orderbook();
    assert!(json["bids"]["100"].is_array());
    assert_eq!(json["bids"]["100"][0]["qty"], "10");
}

#[test]
fn add_resting_sell_order() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(2, "SELL", 5, 200));
    let json = ob.display_orderbook();
    assert!(json["asks"]["200"].is_array());
    assert_eq!(json["asks"]["200"][0]["qty"], "5");
}

#[test]
fn multiple_orders_same_price_fifo() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 5, 100));
    ob.add_resting_order(make_order(2, "BUY", 3, 100));
    let json = ob.display_orderbook();
    assert_eq!(json["bids"]["100"].as_array().unwrap().len(), 2);
    assert_eq!(json["bids"]["100"][0]["qty"], serde_json::json!("5"));
    assert_eq!(json["bids"]["100"][1]["qty"], serde_json::json!("3"));
}

#[tokio::test]
async fn buy_matches_lowest_ask_first() {
    let mut ob = OrderBook::new();
    // Two asks: one at 100, one at 200
    ob.add_resting_order(make_order(1, "SELL", 5, 200));
    ob.add_resting_order(make_order(2, "SELL", 5, 100));

    // Incoming buy at 200 should match the ask at 100 (cheapest)
    let result = ob.engine(make_order(3, "BUY", 5, 200)).await;

    assert_eq!(result.trades.trades.len(), 1);
    let trade = &result.trades.trades[0];
    assert_eq!(trade.buyer_id, 3);
    assert_eq!(trade.seller_id, 2);
    assert_eq!(trade.qty, Decimal::from(5));
    assert_eq!(trade.price, Decimal::from(100));
    assert!(result.appends.is_none());
}

#[tokio::test]
async fn sell_matches_highest_bid_first() {
    let mut ob = OrderBook::new();
    // Two bids: one at 100, one at 200
    ob.add_resting_order(make_order(1, "BUY", 5, 100));
    ob.add_resting_order(make_order(2, "BUY", 5, 200));

    // Incoming sell at 100 should match the bid at 200 (highest)
    let result = ob.engine(make_order(3, "SELL", 5, 100)).await;

    assert_eq!(result.trades.trades.len(), 1);
    let trade = &result.trades.trades[0];
    assert_eq!(trade.buyer_id, 2);
    assert_eq!(trade.seller_id, 3);
    assert_eq!(trade.qty, Decimal::from(5));
    assert_eq!(trade.price, Decimal::from(200));
    assert!(result.appends.is_none());
}

#[tokio::test]
async fn buy_no_match_price_too_low() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 5, 200));

    // Buy at 100 cannot match ask at 200
    let result = ob.engine(make_order(2, "BUY", 5, 100)).await;

    assert_eq!(result.trades.trades.len(), 0);
    assert!(result.appends.is_some());
    assert_eq!(result.appends.unwrap().qty, Decimal::from(5));
}

#[tokio::test]
async fn sell_no_match_price_too_high() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 5, 100));

    // Sell at 200 cannot match bid at 100
    let result = ob.engine(make_order(2, "SELL", 5, 200)).await;

    assert_eq!(result.trades.trades.len(), 0);
    assert!(result.appends.is_some());
    assert_eq!(result.appends.unwrap().qty, Decimal::from(5));
}

#[tokio::test]
async fn partial_fill_buy() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 3, 100));

    // Buy 5 but only 3 available — partial fill
    let result = ob.engine(make_order(2, "BUY", 5, 100)).await;

    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(3));
    assert!(result.appends.is_some());
    assert_eq!(result.appends.unwrap().qty, Decimal::from(2));
}

#[tokio::test]
async fn partial_fill_sell() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 3, 200));

    // Sell 5 but only 3 available — partial fill
    let result = ob.engine(make_order(2, "SELL", 5, 100)).await;

    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(3));
    assert!(result.appends.is_some());
    assert_eq!(result.appends.unwrap().qty, Decimal::from(2));
}

#[tokio::test]
async fn buy_crosses_multiple_ask_levels() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 3, 100));
    ob.add_resting_order(make_order(2, "SELL", 4, 200));

    // Buy 7 at 200 — should fill both levels
    let result = ob.engine(make_order(3, "BUY", 7, 200)).await;

    assert_eq!(result.trades.trades.len(), 2);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(3));
    assert_eq!(result.trades.trades[0].price, Decimal::from(100));
    assert_eq!(result.trades.trades[1].qty, Decimal::from(4));
    assert_eq!(result.trades.trades[1].price, Decimal::from(200));
    assert!(result.appends.is_none());
}

#[tokio::test]
async fn sell_crosses_multiple_bid_levels() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 3, 200));
    ob.add_resting_order(make_order(2, "BUY", 4, 100));

    // Sell 7 at 100 — should fill both levels (highest first)
    let result = ob.engine(make_order(3, "SELL", 7, 100)).await;

    assert_eq!(result.trades.trades.len(), 2);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(3));
    assert_eq!(result.trades.trades[0].price, Decimal::from(200));
    assert_eq!(result.trades.trades[1].qty, Decimal::from(4));
    assert_eq!(result.trades.trades[1].price, Decimal::from(100));
    assert!(result.appends.is_none());
}

#[tokio::test]
async fn fulfilled_orders_cleaned_from_book() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order_with_id(1, 1, "SELL", 5, 100));

    let result = ob.engine(make_order(2, "BUY", 5, 100)).await;

    assert_eq!(result.fulfilled_ids, vec![1]);
    let json = ob.display_orderbook();
    assert_eq!(json["asks"], serde_json::json!({}));
}

#[tokio::test]
async fn set_last_resting_id_buy() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 5, 100));
    ob.set_last_resting_id("BUY", Decimal::from(100), 42);

    let json = ob.display_orderbook();
    assert_eq!(json["bids"]["100"][0]["order_id"], serde_json::json!(42));
}

#[tokio::test]
async fn set_last_resting_id_sell() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 5, 200));
    ob.set_last_resting_id("SELL", Decimal::from(200), 99);

    let json = ob.display_orderbook();
    assert_eq!(json["asks"]["200"][0]["order_id"], 99);
}

#[tokio::test]
async fn exact_fill_no_leftover() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 5, 100));

    let result = ob.engine(make_order(2, "BUY", 5, 100)).await;

    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(5));
    assert!(result.appends.is_none());
    let json = ob.display_orderbook();
    assert_eq!(json["asks"], serde_json::json!({}));
}

#[tokio::test]
async fn buy_at_exact_ask_price() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 5, 150));

    // Buy at exactly 150 — should match
    let result = ob.engine(make_order(2, "BUY", 5, 150)).await;
    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].price, Decimal::from(150));
}

#[tokio::test]
async fn sell_at_exact_bid_price() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "BUY", 5, 150));

    // Sell at exactly 150 — should match
    let result = ob.engine(make_order(2, "SELL", 5, 150)).await;
    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].price, Decimal::from(150));
}

#[tokio::test]
async fn empty_book_buy_becomes_resting() {
    let mut ob = OrderBook::new();
    let result = ob.engine(make_order(1, "BUY", 10, 100)).await;

    assert_eq!(result.trades.trades.len(), 0);
    assert!(result.appends.is_some());
    let appended = result.appends.unwrap();
    assert_eq!(appended.qty, Decimal::from(10));
    assert_eq!(appended.price, Decimal::from(100));

    let json = ob.display_orderbook();
    assert_eq!(json["bids"]["100"][0]["qty"], serde_json::json!("10"));
}

#[tokio::test]
async fn empty_book_sell_becomes_resting() {
    let mut ob = OrderBook::new();
    let result = ob.engine(make_order(1, "SELL", 10, 200)).await;

    assert_eq!(result.trades.trades.len(), 0);
    assert!(result.appends.is_some());

    let json = ob.display_orderbook();
    assert_eq!(json["asks"]["200"][0]["qty"], serde_json::json!("10"));
}

#[tokio::test]
async fn fifo_at_same_price() {
    let mut ob = OrderBook::new();
    ob.add_resting_order(make_order(1, "SELL", 3, 100));
    ob.add_resting_order(make_order(2, "SELL", 2, 100));

    // Buy 3 — should take from order 1 first (FIFO)
    let result = ob.engine(make_order(3, "BUY", 3, 100)).await;

    assert_eq!(result.trades.trades.len(), 1);
    assert_eq!(result.trades.trades[0].seller_id, 1);
    assert_eq!(result.trades.trades[0].qty, Decimal::from(3));

    // Order 2 still has 2 remaining
    let json = ob.display_orderbook();
    assert_eq!(json["asks"]["100"][0]["qty"], serde_json::json!("2"));
}
