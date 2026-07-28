use actix_web::{Error, HttpMessage, dev::ServiceRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use dotenvy::dotenv;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
}

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let secret = std::env::var("JWT_SECRET").expect("JWT SECRET not found");

    let token_data = decode::<Claims>(
        credentials.token(),
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    );
    match token_data {
        Ok(data) => {
            // stash the claims so handlers can pull them out later
            req.extensions_mut().insert(data.claims);
            Ok(req)
        }

        Err(e) => {
            eprintln!("JWT decode error: {:?}", e);
            Err((actix_web::error::ErrorUnauthorized("invalid token"), req))
        }
    }
}
