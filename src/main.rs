use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

use core::*;
use db_access::*;
use handlers::*;
use security::*;
use state::*;

mod core;
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
    let app_state = AppState::new(connect_to_db().await?, jwt_config.clone());

    //// GOVERNORS ////
    let public_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60)
            .burst_size(3)
            .finish()
            .expect("Unable to set up Governor! Server is shutdown!"),
    );

    let private_governor = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(JwtKeyExtractor)
            .per_second(60)
            .burst_size(5)
            .finish()
            .expect("Unable to set up Governor! Server is shutdown!"),
    );

    //Getting RateLimiters of governors and cloning them to send to closure
    let public_limiter = public_governor.limiter().clone();
    let private_limiter = private_governor.limiter().clone();

    //Creating additional tokio task to clean up RateLimiters storage once a day
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        loop {
            interval.tick().await;
            tracing::info!("Starting RateLimiters clean ups...");
            public_limiter.retain_recent();
            private_limiter.retain_recent();
            tracing::info!("Finished RateLimiters clean ups!");
        }
    });

    //Creating additional tokio task to update Secret Every 24-hours

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        loop {
            interval.tick().await;
            tracing::info!("Changing Secret");
            jwt_config.write().await.secret = generate_secret();
            tracing::info!("Finished changing Secret");
        }
    });

    let public_governor_layer = GovernorLayer {
        config: public_governor,
    };

    let private_governor_layer = GovernorLayer {
        config: private_governor,
    };

    //// ROUTERS ////
    let public_router = Router::new()
        .route("/health", get(health_check))
        .route("/login", post(login))
        .layer(public_governor_layer);

    let private_router = Router::new()
        .route("/api/get-scores", get(get_scores))
        .route("/api/set-score", post(commit_record))
        .route("/api/flush", delete(flush))
        .layer(middleware::from_fn({
            let state = app_state.clone();

            move |req, next| {
                let state = state.clone();
                jwt_middleware(req, next, state)
            }
        }))
        .layer(private_governor_layer);

    let app = Router::new()
        .merge(public_router)
        .merge(private_router)
        .fallback(handler_404)
        .layer(middleware::from_fn(set_up_security_headers))
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(RequestBodyLimitLayer::new(1024))
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .with_state(app_state.clone());

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!("Server is up!");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(wait_for_shutdown_signal())
    .await
    .unwrap();

    Ok(())
}
