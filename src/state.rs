use sqlx::PgPool;
use crate::security::JwtConfig;


#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_config: JwtConfig,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_config: JwtConfig) -> Self {
        AppState {
            pool,
            jwt_config,
        }
    }
}
