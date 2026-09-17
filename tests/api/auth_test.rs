use actix_web::{App, web, test};

use trading_engine::auth;
use trading_engine::domain::common::AuthResponseSuccess;

/// Delete a single test user and all their child rows (trades, orders,
/// balances).  Only touches the one user — never does a global delete.
async fn cleanup_user(pool: &sqlx::PgPool, username: &str) {
    if let Ok(Some((uid,))) =
        sqlx::query_as::<_, (i32,)>("SELECT id FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await
    {
        sqlx::query("DELETE FROM trades WHERE buyer_id = $1 OR seller_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM orders WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM balances WHERE user_id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
    }
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
    let pool = super::get_pool();
    App::new()
        .app_data(web::Data::new(Some(pool.clone())))
        .service(auth::register::register_user)
        .service(auth::login::login_user)
}

#[tokio::test]
async fn test_register_success() {
    let pool = super::get_pool();
    let username = &super::unique_user("test_reg_ok");
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
    let pool = super::get_pool();
    let username = &super::unique_user("test_dup");
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
    let pool = super::get_pool();
    let username = &super::unique_user("test_login_ok");
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
    let pool = super::get_pool();
    let username = &super::unique_user("test_login_bad");
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
    let pool = super::get_pool();
    let username = &super::unique_user("test_jwt");
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

    // both tokens decode to valid claims with the correct username
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "test_secret_for_api_tests".to_string());
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());

    let reg_claims: trading_engine::domain::common::Claims =
        jsonwebtoken::decode(&register_token, &key, &validation).unwrap().claims;
    assert_eq!(reg_claims.username.as_str(), username.as_str());

    let login_claims: trading_engine::domain::common::Claims =
        jsonwebtoken::decode(&login_token, &key, &validation).unwrap().claims;
    assert_eq!(login_claims.username.as_str(), username.as_str());

    cleanup_user(pool, username).await;
}
