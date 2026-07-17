use actix_web::{App, HttpServer};
use orderbook::route;

mod orderbook;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Hello, world!");
    HttpServer::new(move || App::new().service(route::hello).service(route::placeOrder))
        .bind(("127.0.0.1", 8081))?
        .run()
        .await
}
