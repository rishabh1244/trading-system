use actix_web::{App, HttpServer, rt, web};
use std::sync::OnceLock;
use sqlx::PgPool;

use crate::api_gateway::db;
use crate::auth;
use crate::orderbook;

const PORT: u16 = 8080;
static DB_POOL: OnceLock<PgPool> = OnceLock::new();

pub async fn api_gateway() -> std::io::Result<()> {
    let database_url = "postgres://localhost/trading_engine".to_string();

    rt::spawn(async move {
        match db::init_pool(&database_url).await {
            Ok(pool) => {
                println!("Database connected successfully");
                let _ = DB_POOL.set(pool);
            }
            Err(e) => eprintln!("Warning: Failed to connect to database: {e}"),
        }
    });

    println!("Trading engine running on http://127.0.0.1:{PORT}");
    HttpServer::new(move || {
        let pool = DB_POOL.get().cloned();
        App::new()
            .app_data(web::Data::new(pool))
            .service(orderbook::route::hello)
            .service(orderbook::route::placeOrder)
            .service(auth::login::login_user)
            .service(auth::register::register_user)
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
