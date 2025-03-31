use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tower_governor::key_extractor::KeyExtractor;

use crate::{error::JwtError, state::AppState};

#[derive(Clone)]
pub struct JwtKeyExtractor;

impl KeyExtractor for JwtKeyExtractor {
    type Key = String;

    fn extract<T>(
        &self,
        req: &axum::http::Request<T>,
    ) -> Result<Self::Key, tower_governor::GovernorError> {
        req.headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| token.to_string())
            .ok_or(tower_governor::errors::GovernorError::UnableToExtractKey)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub _leeway: u64,
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
            _leeway: 60,
            validation,
        }
    }
}

pub async fn jwt_middleware(
    req: Request<Body>,
    next: Next,
    state: AppState,
) -> Result<Response, JwtError> {
    let token = req
        .headers()
        .get("Authorization")
        .ok_or(JwtError::MissingAuthHeader)?
        .to_str()
        .map_err(|_| JwtError::InvalidTokenFormat)?
        .strip_prefix("Bearer ")
        .ok_or(JwtError::InvalidTokenFormat)?
        .trim();

    let _claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_config.secret.as_ref()),
        &state.jwt_config.validation,
    )
    .map_err(|e| JwtError::DecodeError(e))?
    .claims;

    Ok(next.run(req).await)
}

pub fn generate_jwt(user_id: &str, secret: &str) -> Result<String, JwtError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("Invalid timestamp! Server is shutdown!")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        role: "default".into(),
    };

    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| JwtError::_EncodingError(e))
}

pub async fn validate_user(_username: &String, _password: &String) -> Result<User, String> {
    Ok(User {
        id: "default".into(),
    })
}

pub async fn set_up_security_headers(
    req: axum::http::Request<Body>,
    next: axum::middleware::Next,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let mut response = next.run(req).await;

    // Sources of content limited only to our domain
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    );

    // Use only HTTPS in a year (will fail in local)
    response.headers_mut().insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Restict guessing on content type
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // Restrict usage in iframes on other sites, apps
    response
        .headers_mut()
        .insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Restict sending info when of parameters when downgrate from https to http
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer-when-downgrade"),
    );
    // Restrict usage of geolocation, camera
    response.headers_mut().insert(
        "Permissions-Policy",
        HeaderValue::from_static("geolocation=(), camera=()"),
    );

    Ok(response)
}
