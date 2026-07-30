use actix_web::{HttpResponse, Responder, post, web};
use sqlx::PgPool;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};

use crate::domain::common::{AuthData, AuthResponseFailure, AuthResponseSuccess, User, generate_token};

#[post("/api/login")]
pub async fn login_user(pool: web::Data<Option<PgPool>>, req_body: web::Json<AuthData>) -> impl Responder {
    let pool = match pool.get_ref() {
        Some(p) => p,
        None => {
            return HttpResponse::ServiceUnavailable().json(AuthResponseFailure {
                fail_reason: "database not connected".to_string(),
            });
        }
    };

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&req_body.username)
        .fetch_optional(pool)
        .await;

    match user {
        Ok(Some(user)) => {
            let parsed_hash = match PasswordHash::new(&user.password_hash) {
                Ok(h) => h,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(AuthResponseFailure {
                        fail_reason: "invalid password hash".to_string(),
                    });
                }
            };

            if Argon2::default()
                .verify_password(req_body.password.as_bytes(), &parsed_hash)
                .is_ok()
            {
                let token = generate_token(&user.username, user.id);
                HttpResponse::Ok().json(AuthResponseSuccess { token })
            } else {
                HttpResponse::Unauthorized().json(AuthResponseFailure {
                    fail_reason: "invalid credentials".to_string(),
                })
            }
        }
        _ => HttpResponse::Unauthorized().json(AuthResponseFailure {
            fail_reason: "invalid credentials".to_string(),
        }),
    }
}
