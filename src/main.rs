mod MDS;
mod OMS;
mod api_gateway;
mod auth;
mod domain;
mod matching_engine;
mod middleware;
mod trading_engine;
use MDS::socket::SocketServer;
use api_gateway::api;
use dotenvy::dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    std::thread::spawn(|| {
        let server = SocketServer::new();
        if let Err(e) = server.run("127.0.0.1:7878") {
            eprintln!("socket server failed: {e}");
        }
    });

    api::api_gateway().await
}
