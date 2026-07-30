mod OMS;
mod api_gateway;
mod auth;
mod domain;
mod matching_engine;
mod middleware;

use api_gateway::api;
use dotenvy::dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    api::api_gateway().await
}
