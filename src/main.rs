use dotenv::dotenv;
use std::env;

use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::FmtSubscriber;

use db_access::*;
use handlers::*;
use security::*;
use state::*;

mod db_access;
mod error;
mod handlers;
mod security;
mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    set_up_tracing();
    let cors = set_up_cors();
    let jwt_config = set_up_jwt();

    let app_state = AppState {
        pool: connect_to_db().await?,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/get-scores", get(get_scores))
        .route("/api/set-score", post(commit_record))
        .route("/api/flush", delete(flush))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(middleware::from_fn(move |req, next| {
            jwt_middleware(req, next, jwt_config.clone())
        }))
        .layer(cors)
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!("Server is up!");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

fn set_up_jwt() -> JwtConfig {
    dotenv().ok();
    let secret = env::var("JWT_SECRET").expect("Secret not found in .env! Server is shutdown!");
    JwtConfig::new(&secret)
}

fn set_up_tracing() {
    std::fs::create_dir_all("logs").expect(
        "Can't create folder for logs! Logging to file is not working! Server is shutdown!",
    );
    let writer = RollingFileAppender::new(Rotation::DAILY, "logs", "serv.log");

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_writer(writer)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Loggin not ready! Server is shutdown!");
}

fn set_up_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}
