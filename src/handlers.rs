use crate::{
    db_access::{PlayerScore, add_new_score_db, flush_scores_db, get_scores_db},
    error::ServerError,
    state::AppState,
};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use validator::Validate;

pub async fn health_check() -> Json<Value> {
    Json(json!({"status": "OK", "message": "Axum is working fine"}))
}

pub async fn get_scores(State(state): State<AppState>) -> Result<Json<Vec<PlayerScore>>, Response> {
    get_scores_db(&state.pool)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Can't get scores!");
            ServerError::Database(e.to_string()).into_response()})
}

pub async fn flush(State(state): State<AppState>) -> Result<Json<Value>, Response> {
    flush_scores_db(&state.pool)
        .await
        .map(|_| Json(json!({"status": "Ok"})))
        .map_err(|e| {
            tracing::error!("Can't flush scores!");
            ServerError::Database(e.to_string()).into_response()})
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
            ServerError::Database(e.to_string()).into_response()})
}
