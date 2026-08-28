mod MDS;
mod OMS;
mod api_gateway;
mod auth;
mod domain;
mod matching_engine;
mod middleware;
mod trading_engine;
use api_gateway::api;
use api_gateway::db;
use domain::market::{MarketData, SocketServer};
use dotenvy::dotenv;
use matching_engine::orderbook::OrderBook;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use trading_engine::engine::update_orderbook;

async fn startup() -> (PgPool, Arc<Mutex<OrderBook>>) {
    let database_url = std::env::var("DATABASE_URL").expect("database url not found");
    let pool = db::init_pool(&database_url)
        .await
        .expect("db connection failed");

    let mut orderbook = OrderBook::new();
    update_orderbook(&mut orderbook, &pool)
        .await
        .expect("failed to load resting orders from db");

    (pool, Arc::new(Mutex::new(orderbook)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let market = Arc::new(Mutex::new(MarketData::new()));
    let socket_server = Arc::new(SocketServer::new());
    let (pool, orderbook) = startup().await;

    let run_server = socket_server.clone();
    tokio::spawn(async move {
        if let Err(e) = run_server.run("127.0.0.1:7878").await {
            eprintln!("socket server failed: {e}");
        }
    });

    api::api_gateway(socket_server, market, orderbook, pool).await
}
