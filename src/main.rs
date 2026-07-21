use actix_web::{web, App, HttpServer};

mod db;
mod orderbook;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = "postgres://localhost/trading_engine";
    let pool = db::init_pool(database_url)
        .await
        .expect("Failed to create database pool");

    println!("Trading engine running on http://127.0.0.1:8081");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(orderbook::route::hello)
            .service(orderbook::route::placeOrder)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
