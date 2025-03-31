use axum::{http::StatusCode, response::IntoResponse};
use serde_json::json;

#[derive(Debug)]
pub enum ServerError {
    Validation(String),
    Database(String),
    Authentication(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ServerError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                json!({"error:": "Validation failed!", "details:": msg}).to_string(),
            )
                .into_response(),
            ServerError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error:": "Database failed!", "details:": msg}).to_string(),
            )
                .into_response(),
            ServerError::Authentication(msg) => (
                StatusCode::UNAUTHORIZED,
                json!({"error:": "Internal Server Error", "details:": msg}).to_string(),
            )
                .into_response(),
        }
    }
}

impl From<sqlx::error::Error> for ServerError {
    fn from(value: sqlx::error::Error) -> Self {
        ServerError::Database(value.to_string())
    }
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Validation(msg) => write!(f, "Validation error: {}", msg),
            ServerError::Database(msg) => write!(f, "Database error: {}", msg),
            ServerError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for ServerError {}

#[derive(Debug)]
pub enum JwtError {
    MissingAuthHeader,
    InvalidTokenFormat,
    DecodeError(jsonwebtoken::errors::Error),
    _EncodingError(jsonwebtoken::errors::Error),
}

impl IntoResponse for JwtError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            JwtError::MissingAuthHeader => (
                StatusCode::UNAUTHORIZED,
                json!({"error:": "Validation failed!", "details:": "Auth header is missing"})
                    .to_string(),
            ),
            JwtError::InvalidTokenFormat => (
                StatusCode::BAD_REQUEST,
                json!({"error:": "Validation failed!", "details:": "Invalid token format. Expected: Bearer <token>"}).to_string(),
            ),
            JwtError::DecodeError(e) => {
                tracing::warn!("JWT Decode error: {:?}", e);
                (StatusCode::UNAUTHORIZED,
                json!({"error:": "Validation failed!", "details:": "Invalid or expired token"}).to_string())
            }
            JwtError::_EncodingError(e) => {
                tracing::warn!("JWT Encode error: {:?}", e);
                (StatusCode::UNAUTHORIZED,
                json!({"error:": "Validation failed!", "details:": "Invalid or expired token"}).to_string())
            }
        };

        (status, message).into_response()
    }
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::MissingAuthHeader => write!(f, "Auth header is missing"),
            JwtError::InvalidTokenFormat => {
                write!(f, "Invalid token format. Expected: Bearer <token>")
            }
            JwtError::DecodeError(_) => write!(f, "Invalid or expired token"),
            JwtError::_EncodingError(_) => write!(f, "Invalid or expired token"),
        }
    }
}
