//! API routes for the demo server

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::state::AppState;

/// Build API routes
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Info endpoints
        .nest("/info", info_routes())
}

/// Info endpoint routes
fn info_routes() -> Router<AppState> {
    Router::new()
        // Perpetuals metadata
        .route("/perp-dexs", get(get_perp_dexs))
        .route("/meta", get(get_meta))
}

/// GET /api/info/perp-dexs - Get all perpetual DEXs
async fn get_perp_dexs(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.perp_dexs().await {
        Ok(dexs) => Json(serde_json::json!({
            "success": true,
            "data": dexs
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// GET /api/info/meta - Get perpetual metadata
async fn get_meta(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.meta(None).await {
        Ok(meta) => Json(serde_json::json!({
            "success": true,
            "data": meta
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}
