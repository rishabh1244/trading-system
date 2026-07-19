use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
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

fn search_match(order: &Order, order_type: &str) -> i32 {
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

        // check whether prices are compatible
        let price_match = if order_type == "bid" {
            // buyer is willing to pay >= seller's price
            book[i].price <= order.price
        } else {
            // seller is willing to sell <= buyer's price
            book[i].price >= order.price
        };

        if !price_match {
            // since the order book is sorted, nothing after this can match
            break;
        }

        let match_qty = book[i].qty.min(remaining as u64);

        remaining -= match_qty as i64;
        book[i].qty -= match_qty;

        // remove fully filled orders
        if book[i].qty == 0 {
            book.remove(i);
        } else {
            i += 1;
        }
    }

    remaining.max(0) as i32
}

impl OrderBook {
    fn print(&self) {
        let bids = self.bids.lock().unwrap();
        let asks = self.asks.lock().unwrap();

        println!("----- ORDER BOOK -----");
        println!("BIDS:");
        for order in bids.iter() {
            println!("{:?}", order);
        }

        println!("ASKS:");
        for order in asks.iter() {
            println!("{:?}", order);
        }
        println!("----------------------");
    }
}
/*
curl -X POST 'http://127.0.0.1:8081/placeOrder?order_type=ask' \
  -H 'Content-Type: application/json' \
  -d '{"userId":"user123","qty":10,"price":100}'&&
curl -X POST 'http://127.0.0.1:8081/placeOrder?order_type=ask' \
  -H 'Content-Type: application/json' \
  -d '{"userId":"user123","qty":10,"price":101}'
curl -X POST 'http://127.0.0.1:8081/placeOrder?order_type=ask' \
  -H 'Content-Type: application/json' \
  -d '{"userId":"user123","qty":10,"price":102}'

*/

async fn order_book_append(order_type: String, order: Order) {
    let book = if order_type == "bid" {
        &ORDER_BOOK.bids
    } else {
        &ORDER_BOOK.asks
    };
    let mut book = book.lock().unwrap();
    let pos = if order_type == "bid" {
        //increasing order
        book.iter().position(|o| o.price <= order.price)
    } else {
        // decreasing order
        book.iter().position(|o| o.price >= order.price)
    };
    match pos {
        Some(p) => book.insert(p, order),
        None => book.push(order),
    }
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
    let remaining = searchMatch(&order, &query.order_type);

    if remaining > 0 {
        order.qty = remaining as u64;
        order_book_append(query.order_type.clone(), order).await;
        ORDER_BOOK.print();

        return HttpResponse::Ok().body("appended ");
    }

    HttpResponse::Ok().body("working on it")
}

#[post("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("hi")
}
