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

async fn check_if_record_worthy(pool: &PgPool, score: &PlayerScore) -> Result<bool, ServerError> {
    let min_score = sqlx::query_scalar("SELECT COALESCE (MIN(player_score), 1) FROM (SELECT player_score FROM flappy_dragon_score ORDER BY player_score DESC LIMIT 10) AS top")
        .fetch_optional(pool).await?.unwrap_or(1);

    Ok(score.player_score >= min_score)
}

pub async fn add_new_score_db(pool: &PgPool, score: PlayerScore) -> Result<(), ServerError> {
    if check_if_record_worthy(pool, &score).await? {
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

#[cfg(test)]
mod db_tests {
    use serial_test::serial;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_db_add_new_score() {}

    async fn get_test_db_pool() -> PgPool {
        dotenv().ok();
        let path = env::var("TEST_DATABASE_URL").expect("Test db path is not found!");
        PgPool::connect(&path)
            .await
            .expect("Cant connect to test DB!")
    }

    #[tokio::test]
    #[serial]
    async fn test_db_connection() {
        assert!(connect_to_db().await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_db_health_check() {
        let pool = connect_to_db()
            .await
            .expect("Can't connect to database - can't get a pool");
        assert!(health_db(&pool).await.is_ok(), "Health is not Ok");
    }

    #[tokio::test]
    #[serial]
    async fn test_db_flush() {
        let pool = get_test_db_pool().await;
        pool.begin().await.expect("Transaction failed!");

        assert!(
            flush_scores_db(&pool).await.is_ok(),
            "Flush is not worked out!"
        );

        let scores = get_scores_db(&pool)
            .await
            .expect("Failed to fetch data after flush!");
        assert!(scores.is_empty(), "Scores are not empty after flush!");
    }

    #[tokio::test]
    #[serial]
    async fn test_db_get_scores() {
        let pool = get_test_db_pool().await;
        flush_scores_db(&pool).await.expect("Couldn't flush db!");
        assert!(get_scores_db(&pool).await.is_ok(), "Can't get scores!");
        assert!(
            add_new_score_db(
                &pool,
                PlayerScore {
                    player_name: "Bobby".to_string(),
                    player_score: 50
                }
            )
            .await
            .is_ok(),
            "Can't add new score"
        );

        let score: Vec<PlayerScore> = get_scores_db(&pool).await.expect("Can't get scores!");
        println!("{:?}", score);
        assert!(score.len() == 1, "Wrong population!");
    }

    #[tokio::test]
    #[serial]
    async fn test_db_is_worthy() {
        let pool = get_test_db_pool().await;
        flush_scores_db(&pool).await.expect("Can't flush db!");
        let pre_player_zero = check_if_record_worthy(
            &pool,
            &PlayerScore {
                player_name: "Max".to_string(),
                player_score: 0,
            },
        )
        .await
        .expect("Cant check DB");
        let pre_player_one = check_if_record_worthy(
            &pool,
            &PlayerScore {
                player_name: "Max".to_string(),
                player_score: 1,
            },
        )
        .await
        .expect("Cant check DB");

        assert!(!pre_player_zero);
        assert!(pre_player_one);

        for i in 1..11 {
            sqlx::query!(
                "INSERT INTO flappy_dragon_score (player_name, player_score) VALUES ($1, $2)",
                "TestMike",
                i,
            )
            .execute(&pool)
            .await
            .expect("Cant insert test data!");
        }

        let first_player = check_if_record_worthy(
            &pool,
            &PlayerScore {
                player_name: "Max".to_string(),
                player_score: 0,
            },
        )
        .await
        .expect("Cant check DB");
        let second_player = check_if_record_worthy(
            &pool,
            &PlayerScore {
                player_name: "Max".to_string(),
                player_score: 1,
            },
        )
        .await
        .expect("Cant check DB");
        let third_player = check_if_record_worthy(
            &pool,
            &PlayerScore {
                player_name: "Max".to_string(),
                player_score: 10,
            },
        )
        .await
        .expect("Cant check DB");
        let fourth_player = check_if_record_worthy(
            &pool,
            &PlayerScore {
                player_name: "Max".to_string(),
                player_score: 11,
            },
        )
        .await
        .expect("Cant check DB");
        assert!(!first_player, "First");
        assert!(second_player, "Second");
        assert!(third_player, "Third");
        assert!(fourth_player, "Fourth");

        //Clearing up test db after tests
        flush_scores_db(&pool)
            .await
            .expect("Cant flush test database!");
    }
}
