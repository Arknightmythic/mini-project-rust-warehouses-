pub mod auth_routes;
pub mod role_routes;
pub mod user_routes;
pub mod warehouse_routes;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api/auth", auth_routes::router())
        .nest("/api/users", user_routes::router())
        .nest("/api/roles", role_routes::router())
        .nest("/api/warehouses", warehouse_routes::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
