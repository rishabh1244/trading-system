use actix_web::{HttpResponse, Responder, post, web};
use sqlx::PgPool;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use super::common::{AuthData, AuthResponseFailure, AuthResponseSuccess, generate_token};

#[post("/api/register")]
pub async fn register_user(pool: web::Data<Option<PgPool>>, req_body: web::Json<AuthData>) -> impl Responder {
    let pool = match pool.get_ref() {
        Some(p) => p,
        None => {
            return HttpResponse::ServiceUnavailable().json(AuthResponseFailure {
                fail_reason: "database not connected".to_string(),
            });
        }
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req_body.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    let result = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2)")
        .bind(&req_body.username)
        .bind(&password_hash)
        .execute(pool)
        .await;

    match result {
        Ok(_) => {
            let token = generate_token(&req_body.username);
            HttpResponse::Ok().json(AuthResponseSuccess { token })
        }
        Err(e) => HttpResponse::InternalServerError().json(AuthResponseFailure {
            fail_reason: e.to_string(),
        }),
    }
}
