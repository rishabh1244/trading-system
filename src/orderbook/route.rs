use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Order {
    userId: String,
    qty: u64,
    price: u64,
}

static mut bids: Vec<i32> = Vec::new();
static mut asks: Vec<i32> = Vec::new();

async fn searchMatch(order: &Order) -> i32 {
    9
}

async fn order_book_append(order_type: String, order: &Order) {
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
    let order = order.into_inner();
    let remaining = searchMatch(&order).await;

    if remaining > 0 {
        order_book_append(query.order_type.clone(), &order).await;
        return HttpResponse::Ok().body("appended ");
    }

    HttpResponse::Ok().body("working on it")
}

#[post("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("hi")
}
