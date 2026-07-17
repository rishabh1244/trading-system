use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct Order {
    userId: String,
    qty: u64,
    price: u64,
}

struct OrderBook {
    bids: Mutex<Vec<Order>>,
    asks: Mutex<Vec<Order>>,
}

static ORDER_BOOK: OrderBook = OrderBook {
    bids: Mutex::new(Vec::new()),
    asks: Mutex::new(Vec::new()),
};

async fn searchMatch(order: &Order, order_type: &str) -> i32 {
    let opposite_book = if order_type == "bid" {
        &ORDER_BOOK.asks
    } else {
        &ORDER_BOOK.bids
    };

    let mut book = opposite_book.lock().unwrap();
    let mut remaining = order.qty as i64;
    let mut i = 0;

    while i < book.len() {
        if remaining <= 0 {
            break;
        }

        let price_match = if order_type == "bid" {
            book[i].price <= order.price
        } else {
            book[i].price >= order.price
        };

        if !price_match {
            i += 1;
            continue;
        }

        let match_qty = book[i].qty.min(remaining as u64);
        remaining -= match_qty as i64;
        book[i].qty -= match_qty;

        if book[i].qty == 0 {
            book.swap_remove(i);
        } else {
            i += 1;
        }
    }

    remaining.max(0) as i32
}

async fn order_book_append(order_type: String, order: Order) {
    let book = if order_type == "bid" {
        &ORDER_BOOK.bids
    } else {
        &ORDER_BOOK.asks
    };
    book.lock().unwrap().push(order);
}

#[derive(Deserialize)]
struct PlaceOrderQuery {
    order_type: String,
}

#[post("/placeOrder")]
pub async fn placeOrder(
    order: web::Json<Order>,
    query: web::Query<PlaceOrderQuery>,
) -> impl Responder {
    let mut order = order.into_inner();
    let remaining = searchMatch(&order, &query.order_type).await;

    if remaining > 0 {
        order.qty = remaining as u64;
        order_book_append(query.order_type.clone(), order).await;
        return HttpResponse::Ok().body("appended ");
    }

    HttpResponse::Ok().body("working on it")
}

#[post("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("hi")
}
