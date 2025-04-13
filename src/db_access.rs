use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use validator::Validate;

use crate::error::ServerError;

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize, Validate, PartialEq, Clone)]
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
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_db_add_new_score() {
        let pool = get_test_db_pool().await;
        flush_scores_db(&pool).await.expect("Can't flush test db!");

        let mut players_vector: Vec<PlayerScore> = vec![];

        for i in 0..12 {
            let player = PlayerScore {
                player_name: "Dull".to_string(),
                player_score: i,
            };

            if i > 0 && i <= 10 {
                players_vector.insert(0, player.clone());
            };

            add_new_score_db(&pool, player.clone())
                .await
                .expect("Can't add player to test DB!");

            if i > 0 && i <= 10 {
                assert_eq!(
                    players_vector,
                    get_scores_db(&pool)
                        .await
                        .expect("Can't get scores from DB!"),
                    "Vectors of players doesn't match!"
                );
            }
        }

        assert!(
            get_scores_db(&pool)
                .await
                .expect("Can't get scores from test DB!")
                .len()
                == 10,
            "Final length is more than 10!"
        );

        flush_scores_db(&pool).await.expect("Can't flush test db!");

        //Let's check if right entries is deleted//

        assert!(get_scores_db(&pool)
            .await
            .expect("Can't get scores from test DB!")
            .is_empty());

        let mut players_vector = vec![];

        for i in 1..11 {
            let player = PlayerScore {
                player_name: "Dull".to_string(),
                player_score: i,
            };

            players_vector.insert(0, player.clone());
            add_new_score_db(&pool, player.clone())
                .await
                .expect("Can't add new score to test DB!");
        }
        assert_eq!(
            players_vector,
            get_scores_db(&pool)
                .await
                .expect("Can't get scores from DB!"),
            "Vectors of players doesn't match!"
        );

        let db_scores: Vec<PlayerScore> = get_scores_db(&pool)
            .await
            .expect("Can't get scores from test DB!");

        let first_db_player = db_scores.first().expect("Can't get first one!").clone();
        let last_db_player = db_scores.last().expect("Can't get last one!").clone();

        let first_vec_player = players_vector
            .first()
            .expect("Can't get first one!")
            .clone();
        let last_vec_player = players_vector.last().expect("Can't get last one!").clone();

        assert!(first_vec_player.player_score == 10);
        assert_eq!(first_db_player, first_vec_player);
        assert!(last_db_player.player_score == 1);
        assert_eq!(last_db_player, last_vec_player);

        //Now adding one on top!
        players_vector.insert(
            0,
            PlayerScore {
                player_name: "Dull".to_string(),
                player_score: 10,
            },
        );

        add_new_score_db(
            &pool,
            PlayerScore {
                player_name: "Dull".to_string(),
                player_score: 10,
            },
        )
        .await
        .expect("Can't add score!");

        players_vector.pop();

        let db_scores: Vec<PlayerScore> = get_scores_db(&pool)
            .await
            .expect("Can't get scores from test DB!");

        let first_db_player = db_scores.first().expect("Can't get first one!").clone();
        let second_db_player = db_scores.get(1).expect("Can't get second one!").clone();
        let last_db_player = db_scores.last().expect("Can't get last one!").clone();

        let first_vec_player = players_vector
            .first()
            .expect("Can't get first one!")
            .clone();
        let second_vec_player = players_vector
            .get(1)
            .expect("Can't get second one!")
            .clone();
        let last_vec_player = players_vector.last().expect("Can't get last one!").clone();

        assert!(first_vec_player.player_score == 10);
        assert_eq!(first_db_player, first_vec_player);
        assert!(last_db_player.player_score == 2);
        assert_eq!(last_db_player, last_vec_player);
        assert!(second_db_player.player_score == 10);
        assert_eq!(second_db_player, second_vec_player);

        flush_scores_db(&pool)
            .await
            .expect("Can't flush scores in test DB!");
    }

    async fn populate_db_with_mock_data(pool: &PgPool, range: std::ops::Range<i32>) {
        for i in range {
            sqlx::query!(
                "INSERT INTO flappy_dragon_score (player_name, player_score) VALUES ($1, $2)",
                "TestMike",
                i,
            )
            .execute(pool)
            .await
            .expect("Cant insert test data!");
        }
    }

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

        populate_db_with_mock_data(&pool, 1..11).await;

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
