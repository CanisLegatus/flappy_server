use crate::Arc;
use crate::JwtConfig;
use crate::generate_secret;
use axum::http::Method;
use std::time::Duration;

#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::FmtSubscriber;

pub async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term_signal = signal(SignalKind::terminate()).unwrap();
        let mut term_hup = signal(SignalKind::hangup()).unwrap();
        let mut term_interrupt = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = term_signal.recv() => {
                tracing::info!("TERM Signal Recieved... Starting graceful shutdown...")
            },
            _ = term_hup.recv() => {
                tracing::info!("HUP Signal Recieved... Starting graceful shutdown...")
            },
            _ = term_interrupt.recv() => {
                tracing::info!("INTERRUPT Signal Recieved... Starting graceful shutdown...")
            },

        }
    }

    #[cfg(windows)]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to set up Ctrl+C handler");
        tracing::info!("Ctrl+C received, starting graceful shutdown...");
    }
}

pub fn set_up_jwt() -> Arc<RwLock<JwtConfig>> {
    Arc::new(RwLock::new(JwtConfig::new(generate_secret())))
}

pub fn set_up_tracing() {
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

pub fn set_up_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "http://0.0.0.0:3000".parse().unwrap(),
            "http://0.0.0.0:8080".parse().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(false)
        .max_age(Duration::from_secs(86400))
}
