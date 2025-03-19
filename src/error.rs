use axum::{http::StatusCode, response::IntoResponse};
use serde_json::json;

pub enum ServerError {
    Validation(String),
    Database(String),
    Authentification(String),
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
            ServerError::Authentification(msg) => (
                StatusCode::UNAUTHORIZED,
                json!({"error:": "Internal Server Error", "details:": msg}).to_string(),
            ).into_response()
        }
    }
}
