mod MDS;
mod OMS;
mod api_gateway;
mod auth;
mod domain;
mod matching_engine;
mod middleware;
mod trading_engine;
use api_gateway::api;
use domain::market::{MarketData, SocketServer};
use dotenvy::dotenv;
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let market = Arc::new(Mutex::new(MarketData::new()));
    let socket_server = Arc::new(SocketServer::new());

    let run_server = socket_server.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_server.run("127.0.0.1:7878") {
            eprintln!("socket server failed: {e}");
        }
    });

    api::api_gateway(socket_server, market).await
}
