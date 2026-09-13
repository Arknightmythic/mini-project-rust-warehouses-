mod configs;
mod middlewares;
mod models;
mod repositories;
mod routes;
mod state;

use std::sync::Arc;

use configs::{AppConfig, database};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    wms_core::telemetry::init("legacy-monolith");

    let config = AppConfig::from_env();
    let pool = database::connect().await?;

    let server_url = config.server_url.clone();
    let server_port = config.server_port;

    let state = AppState {
        pool,
        config: Arc::new(config),
    };

    let app = routes::build_router(state);

    let addr = format!("{server_url}:{server_port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
