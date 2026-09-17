use actix_web::{App, web, test};
use sqlx::PgPool;
use std::sync::OnceLock;

use trading_engine::auth;
use trading_engine::domain::common::AuthResponseSuccess;

static DB_POOL: OnceLock<PgPool> = OnceLock::new();

fn get_pool() -> &'static PgPool {
    DB_POOL.get_or_init(|| {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost:5432/trading_engine".to_string());
        sqlx::PgPool::connect_lazy(&database_url).expect("failed to connect to database")
    })
}

async fn cleanup_user(pool: &PgPool, username: &str) {
    let _ = sqlx::query("DELETE FROM balances WHERE user_id = (SELECT id FROM users WHERE username = $1)")
        .bind(username)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE username = $1")
        .bind(username)
        .execute(pool)
        .await;
}

fn test_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let pool = get_pool();
    App::new()
        .app_data(web::Data::new(Some(pool.clone())))
        .service(auth::register::register_user)
        .service(auth::login::login_user)
}

#[tokio::test]
async fn test_register_success() {
    let pool = get_pool();
    let username = "test_register_user_1";
    cleanup_user(pool, username).await;

    let app = test::init_service(test_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/register")
        .set_json(serde_json::json!({
            "username": username,
            "password": "secure_password_123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "register should return 200");

    let body: AuthResponseSuccess = test::read_body_json(resp).await;
    assert!(!body.token.is_empty(), "token should not be empty");

    cleanup_user(pool, username).await;
}

#[tokio::test]
async fn test_register_duplicate_username() {
    let pool = get_pool();
    let username = "test_register_dup_1";
    cleanup_user(pool, username).await;

    let app = test::init_service(test_app()).await;

    // first register
    let req = test::TestRequest::post()
        .uri("/api/register")
        .set_json(serde_json::json!({
            "username": username,
            "password": "password123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // duplicate register
    let req = test::TestRequest::post()
        .uri("/api/register")
        .set_json(serde_json::json!({
            "username": username,
            "password": "password456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 500, "duplicate register should fail");

    cleanup_user(pool, username).await;
}

#[tokio::test]
async fn test_login_success() {
    let pool = get_pool();
    let username = "test_login_user_1";
    let password = "my_secret_pass";
    cleanup_user(pool, username).await;

    let app = test::init_service(test_app()).await;

    // register first
    let req = test::TestRequest::post()
        .uri("/api/register")
        .set_json(serde_json::json!({
            "username": username,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // login with correct credentials
    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(serde_json::json!({
            "username": username,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "login should return 200");

    let body: AuthResponseSuccess = test::read_body_json(resp).await;
    assert!(!body.token.is_empty(), "token should not be empty");

    cleanup_user(pool, username).await;
}

#[tokio::test]
async fn test_login_wrong_password() {
    let pool = get_pool();
    let username = "test_login_wrong_pw_1";
    cleanup_user(pool, username).await;

    let app = test::init_service(test_app()).await;

    // register
    let req = test::TestRequest::post()
        .uri("/api/register")
        .set_json(serde_json::json!({
            "username": username,
            "password": "correct_password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // login with wrong password
    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(serde_json::json!({
            "username": username,
            "password": "wrong_password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "wrong password should return 401");

    cleanup_user(pool, username).await;
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let app = test::init_service(test_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(serde_json::json!({
            "username": "nonexistent_user_xyz_999",
            "password": "doesnt_matter"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "nonexistent user should return 401");
}

#[tokio::test]
async fn test_register_then_login_returns_valid_jwt() {
    let pool = get_pool();
    let username = "test_jwt_flow_1";
    let password = "jwt_test_pass";
    cleanup_user(pool, username).await;

    let app = test::init_service(test_app()).await;

    // register
    let req = test::TestRequest::post()
        .uri("/api/register")
        .set_json(serde_json::json!({
            "username": username,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let register_body: AuthResponseSuccess = test::read_body_json(resp).await;
    let register_token = register_body.token;

    // login
    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(serde_json::json!({
            "username": username,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let login_body: AuthResponseSuccess = test::read_body_json(resp).await;
    let login_token = login_body.token;

    // both tokens should be valid JWTs (3 parts separated by dots)
    assert_eq!(register_token.split('.').count(), 3, "register token should be a valid JWT");
    assert_eq!(login_token.split('.').count(), 3, "login token should be a valid JWT");

    // tokens should be different (different iat/exp)
    assert_ne!(register_token, login_token, "register and login tokens should differ");

    cleanup_user(pool, username).await;
}
