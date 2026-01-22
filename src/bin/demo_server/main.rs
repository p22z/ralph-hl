//! Demo server for the Hyperliquid Rust SDK
//!
//! A Bloomberg-style demo website showcasing all SDK functionality.
//! Run with: `cargo run --bin demo-server --features demo`

use axum::{
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "demo_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create application state with SDK client
    let state = AppState::new();

    // Configure CORS - allow any origin for demo purposes
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application router
    let app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API routes
        .nest("/api", routes::api_routes())
        // Static file serving for frontend
        .nest_service("/", ServeDir::new("static").append_index_html_on_directories(true))
        // Add middleware
        .layer(cors)
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Demo server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "hyperliquid-demo-server",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
