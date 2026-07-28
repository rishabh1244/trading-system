mod api_gateway;
mod auth;
mod middleware;
mod orderbook;
use api_gateway::api;
use dotenvy::dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    api::api_gateway().await
}
