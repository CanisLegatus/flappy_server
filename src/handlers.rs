use crate::{
    db_access::{PlayerScore, add_new_score_db, flush_scores_db, get_scores_db, health_db},
    error::ServerError,
    security::{generate_jwt, validate_user},
    state::AppState,
};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use validator::Validate;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

/////////////////////////////////// HANDLERS ///////////////////////////////////

pub async fn health_check(State(state): State<AppState>) -> Json<Value> {
    let db_health: String = health_db(&state.pool)
        .await
        .map_or("DOWN".into(), |_| "OK".into());

    Json(json!({"status": "OK",
    "services": {
        "server": "OK",
        "database": db_health
    }}))
}

pub async fn login(
    State(state): State<AppState>,
    Json(credentials): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, Response> {
    let user = validate_user(&credentials.username, &credentials.password)
        .await
        .map_err(|e| {
            tracing::warn!("User is not validated!");
            (ServerError::_Authentification(e.to_string())).into_response()
        })?;

    let secret = state.jwt_config.secret;
    let token = generate_jwt(&user.id, &secret).map_err(|e| {
        tracing::warn!("Can't generate JWT Token!");
        (ServerError::_Authentification(e.to_string())).into_response()
    })?;

    Ok(Json(LoginResponse { token }))
}

pub async fn get_scores(State(state): State<AppState>) -> Result<Json<Vec<PlayerScore>>, Response> {
    get_scores_db(&state.pool).await.map(Json).map_err(|e| {
        tracing::error!("Can't get scores!");
        ServerError::Database(e.to_string()).into_response()
    })
}

pub async fn flush(State(state): State<AppState>) -> Result<Json<Value>, Response> {
    flush_scores_db(&state.pool)
        .await
        .map(|_| Json(json!({"status": "Ok"})))
        .map_err(|e| {
            tracing::error!("Can't flush scores!");
            ServerError::Database(e.to_string()).into_response()
        })
}

pub async fn commit_record(
    State(state): State<AppState>,
    Json(record): Json<PlayerScore>,
) -> Result<Json<Value>, Response> {
    if let Err(e) = record.validate() {
        tracing::error!("Validation of commited score data failed!");
        return Err(ServerError::Validation(format!(
            "{} - Fields errors: {:?}",
            e,
            e.field_errors()
        ))
        .into_response());
    }

    add_new_score_db(&state.pool, record)
        .await
        .map(|_| Json(json!({"status": "Ok"})))
        .map_err(|e| {
            tracing::error!("Adding new score error!");
            ServerError::Database(e.to_string()).into_response()
        })
}
