mod api_gateway;
mod auth;
mod orderbook;

use api_gateway::api;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    api::api_gateway().await
}
