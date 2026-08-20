use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthData {
    pub username: String,
    pub password: String,
}
#[derive(Clone, sqlx::FromRow)]
pub struct User {
    pub username: String,
    pub id: i32,
    pub password_hash: String,
}

#[derive(Clone, Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Balances {
    pub user_id: i32,
    pub balance_btc: Decimal,
    pub balance_inr: Decimal,
    pub reserved_inr: Decimal,
    pub reserved_btc: Decimal,
}

#[derive(Serialize)]
pub struct AuthResponseSuccess {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthResponseFailure {
    pub fail_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
    pub id: i32,
}

pub fn generate_token(username: &str, user_id: i32) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "my_secret_key".to_string());

    let exp = (Utc::now().timestamp() + 3600) as usize;
    let claims = Claims {
        username: username.to_string(),
        id: user_id,
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}
