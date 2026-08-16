use actix_web::{HttpResponse, Responder, post, web};
use sqlx::{PgPool, Row};

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use crate::domain::common::{AuthData, AuthResponseFailure, AuthResponseSuccess, generate_token};

#[post("/api/register")]
pub async fn register_user(
    pool: web::Data<Option<PgPool>>,
    req_body: web::Json<AuthData>,
) -> impl Responder {
    let pool = match pool.get_ref() {
        Some(p) => p,
        None => {
            return HttpResponse::ServiceUnavailable().json(AuthResponseFailure {
                fail_reason: "database not connected".to_string(),
            });
        }
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match Argon2::default().hash_password(req_body.password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => {
            return HttpResponse::InternalServerError().json(AuthResponseFailure {
                fail_reason: format!("password hashing failed: {e}"),
            });
        }
    };

    let result = sqlx::query(
        "INSERT INTO users (username, password_hash)
     VALUES ($1, $2)
     RETURNING id",
    )
    .bind(&req_body.username)
    .bind(&password_hash)
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => {
            let user_id: i32 = row.get("id");

            let _ = sqlx::query(
                "INSERT INTO balances (user_id, balance_btc, balance_inr) VALUES ($1, 0, 0)",
            )
            .bind(user_id)
            .execute(pool)
            .await;

            let token = match generate_token(&req_body.username, user_id) {
                Ok(t) => t,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(AuthResponseFailure {
                        fail_reason: format!("token generation failed: {e}"),
                    });
                }
            };
            HttpResponse::Ok().json(AuthResponseSuccess { token })
        }
        Err(e) => HttpResponse::InternalServerError().json(AuthResponseFailure {
            fail_reason: e.to_string(),
        }),
    }
}
