use std::net::SocketAddr;

use axum::{Router, extract::State, http::StatusCode, routing::get};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://taskion.db".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState { db: pool.clone() };

    let app = Router::new().route("/health", get(health))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

async fn health(State(state): State<AppState>) -> StatusCode {
    match sqlx::query("select 1").execute(&state.db).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            error!("health check failed: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}