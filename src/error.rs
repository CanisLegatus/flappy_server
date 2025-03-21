use axum::{http::StatusCode, response::IntoResponse};
use serde_json::json;

pub enum ServerError {
    Validation(String),
    Database(String),
    _Authentification(String),
    _Internal(String),
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
            ServerError::_Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error:": "Internal Server Error", "details:": msg}).to_string(),
            )
                .into_response(),
            ServerError::_Authentification(msg) => (
                StatusCode::UNAUTHORIZED,
                json!({"error:": "Internal Server Error", "details:": msg}).to_string(),
            )
                .into_response(),
        }
    }
}

#[derive(Debug)]
pub enum JwtError {
    MissingAuthHeader,
    InvalidTokenFormat,
    DecodeError(jsonwebtoken::errors::Error),
}

impl IntoResponse for JwtError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            JwtError::MissingAuthHeader => (StatusCode::UNAUTHORIZED, "Auth header is missing!"),
            JwtError::InvalidTokenFormat => (
                StatusCode::BAD_REQUEST,
                "Invalid token format. Expected: Bearer <token>",
            ),
            JwtError::DecodeError(e) => {
                tracing::warn!("JWT Decode error: {:?}", e);
                (StatusCode::UNAUTHORIZED, "Invalid or expired token")
            }
        };

        (status, message).into_response()
    }
}
