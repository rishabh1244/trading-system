use crate::OMS::order_management::{display_orderbook, fetch_order};
use crate::api_gateway::db;
use crate::auth;
use crate::domain::market::MarketData;
use crate::matching_engine::orderbook::OrderBook;
use crate::middleware::auth_middleware::validator;

use std::sync::{Arc, Mutex};

use actix_web::{App, HttpServer, web};
use actix_web_httpauth::middleware::HttpAuthentication;

const PORT: u16 = 8080;

pub async fn api_gateway() -> std::io::Result<()> {
    let database_url = std::env::var("DATABASE_URL").expect("database url not found");

    let orderbook = Arc::new(Mutex::new(OrderBook::new()));

    let market = Arc::new(Mutex::new(MarketData::new()));

    let pool = db::init_pool(&database_url)
        .await
        .expect("db connection failed");

    // reload resting orders from the database back into the in-memory orderbook
    {
        let mut ob = orderbook.lock().unwrap();
        if let Err(e) = ob.load_from_db(&pool).await {
            eprintln!("failed to load resting orders from db: {e}");
        }
    }

    println!("Trading engine running on http://127.0.0.1:{PORT}");
    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(validator);
        App::new()
            .app_data(web::Data::new(Some(pool.clone())))
            .app_data(web::Data::new(orderbook.clone()))
            .app_data(web::Data::new(market.clone()))
            .service(auth::login::login_user)
            .service(auth::register::register_user)
            .service(
                web::scope("")
                    .wrap(auth)
                    .service(fetch_order)
                    .service(display_orderbook),
            )
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
