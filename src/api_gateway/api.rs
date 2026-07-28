use crate::OMS::order_management::fetch_order;
use crate::api_gateway::db;
use crate::auth;
use crate::middleware::auth_middleware::validator;
use crate::orderbook;

use actix_web::{App, HttpServer, web};
use actix_web_httpauth::middleware::HttpAuthentication;
const PORT: u16 = 8080;

pub async fn api_gateway() -> std::io::Result<()> {
    let database_url = std::env::var("DATABASE_URL").expect("database url not found");

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
            .service(web::scope("").wrap(auth).service(fetch_order))
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
/*
eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6Im5ld3VzZXIiLCJleHAiOjE3ODUyMzU5Njd9.eA3Pwhs-E_Aw9yJ7Thd7x_gYkOTvhfSdIfWaVK_DoEg


curl -X POST 'http://127.0.0.1:8080/api/order' \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6Im5ld3VzZXIiLCJleHAiOjE3ODUyMzU5Njd9.eA3Pwhs-E_Aw9yJ7Thd7x_gYkOTvhfSdIfWaVK_DoEg" \
  -d '{"username":"testuser","password":"testpass"}'

*/
