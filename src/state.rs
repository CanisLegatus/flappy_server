use sqlx::PgPool;

use crate::security::JwtConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_config: JwtConfig,
}
