use std::sync::Arc;

use crate::security::JwtConfig;
use sqlx::PgPool;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_config: Arc<RwLock<JwtConfig>>,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_config: Arc<RwLock<JwtConfig>>) -> Self {
        AppState { pool, jwt_config }
    }
}
