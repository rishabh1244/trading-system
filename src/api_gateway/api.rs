use crate::api_gateway::db;
use crate::auth;
use crate::middleware::auth_middleware::validator;
use crate::orderbook;
use actix_web::{App, HttpServer, web};
use actix_web_httpauth::middleware::HttpAuthentication;
const PORT: u16 = 8080;

pub async fn api_gateway() -> std::io::Result<()> {
    let database_url = "postgres://localhost:5432/trading_engine".to_string();

    let pool = db::init_pool(&database_url)
        .await
        .expect("db connection failed");

    println!("Trading engine running on http://127.0.0.1:{PORT}");
    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(validator);
        App::new()
            .app_data(web::Data::new(Some(pool.clone())))
            .service(orderbook::route::hello)
            .service(orderbook::route::placeOrder)
            .service(auth::login::login_user)
            .service(auth::register::register_user)
        //.service(web::scope("").wrap(auth).service(executeCode))
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
