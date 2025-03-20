use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;

use crate::error::JwtError;

#[derive(Debug, Deserialize)]
struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub leeway: u64,
    pub validation: Validation,
}

impl JwtConfig {
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::default();
        validation.leeway = 60;
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Self {
            secret: secret.to_string(),
            leeway: 60,
            validation,
        }
    }
}

pub async fn jwt_middleware(
    req: Request<Body>,
    next: Next,
    jwt_config: JwtConfig,
) -> Result<Response, JwtError> {
    let token = req
        .headers()
        .get("Authorisation")
        .ok_or(JwtError::MissingAuthHeader)?
        .to_str()
        .map_err(|_| JwtError::InvalidTokenFormat)?
        .strip_prefix("Bearer ")
        .ok_or(JwtError::InvalidTokenFormat)?
        .trim();

    let _claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_ref()),
        &jwt_config.validation,
    )
    .map_err(|e| JwtError::DecodeError(e))?
    .claims;

    Ok(next.run(req).await)
}
