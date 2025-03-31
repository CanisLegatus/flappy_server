use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use validator::Validate;

use crate::error::ServerError;

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize, Validate)]
pub struct PlayerScore {
    #[validate(length(min = 3, max = 20))]
    pub player_name: String,

    #[validate(range(min = 0, max = 1_000_000))]
    pub player_score: i32,
}

pub async fn health_db(pool: &PgPool) -> Result<(), ServerError> {
    sqlx::query!("SELECT 1 AS one")
        .fetch_one(pool)
        .await
        .map_or_else(|e| Err(ServerError::Database(format!("{}", e))), |_| Ok(()))
}

pub async fn connect_to_db() -> Result<PgPool, ServerError> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("Adress not found in .env!");

    let pool = PgPool::connect(&database_url).await?;
    Ok(pool)
}

pub async fn flush_scores_db(pool: &PgPool) -> Result<(), ServerError> {
    sqlx::query!("TRUNCATE TABLE flappy_dragon_score RESTART IDENTITY")
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_scores_db(pool: &PgPool) -> Result<Vec<PlayerScore>, ServerError> {
    let scores_array = sqlx::query_as!(
        PlayerScore,
        "SELECT player_name, player_score FROM flappy_dragon_score ORDER BY player_score DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(scores_array)
}

async fn check_if_record_wothy(pool: &PgPool, score: &PlayerScore) -> Result<bool, ServerError> {
    let min_score = sqlx::query_scalar("SELECT COALESCE (MIN(player_score), 0) FROM (SELECT player_score FROM flappy_dragon_score ORDER BY player_score DESC LIMIT 10) AS top")
        .fetch_optional(pool).await?.unwrap_or(0);

    Ok(score.player_score > min_score)
}

pub async fn add_new_score_db(pool: &PgPool, score: PlayerScore) -> Result<(), ServerError> {
    if check_if_record_wothy(pool, &score).await? {
        // Inserting value
        sqlx::query!(
            "INSERT INTO flappy_dragon_score (player_name, player_score) VALUES ($1, $2)",
            score.player_name,
            score.player_score
        )
        .execute(pool)
        .await?;

        sqlx::query!("DELETE FROM flappy_dragon_score WHERE id NOT IN (SELECT id FROM flappy_dragon_score ORDER BY player_score DESC LIMIT 10)")
            .execute(pool)
            .await?;
    }

    Ok(())
}
