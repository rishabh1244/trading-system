use actix_web::{App, HttpServer, rt};

use crate::api_gateway::db;
use crate::orderbook;

const PORT: u16 = 8080;

pub async fn api_gateway() -> std::io::Result<()> {
    let database_url = "postgres://localhost/trading_engine".to_string();

    rt::spawn(async move {
        match db::init_pool(&database_url).await {
            Ok(pool) => println!("Database connected successfully"),
            Err(e) => eprintln!("Warning: Failed to connect to database: {e}"),
        }
    });

    println!("Trading engine running on http://127.0.0.1:{PORT}");
    HttpServer::new(move || {
        App::new()
            .service(orderbook::route::hello)
            .service(orderbook::route::placeOrder)
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
