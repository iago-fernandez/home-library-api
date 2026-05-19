use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}

pub fn create_jwt(user_id: Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "fallback_secret_change_in_production".to_string());
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(365))
        .expect("Valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiration,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.starts_with("Bearer "))
            .map(|value| value.trim_start_matches("Bearer "));

        let token = auth_header.ok_or((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header"))?;
        let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "fallback_secret_change_in_production".to_string());

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        Ok(token_data.claims)
    }
}