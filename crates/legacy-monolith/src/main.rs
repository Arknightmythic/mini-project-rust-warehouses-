mod configs;
mod middlewares;
mod models;
mod repositories;
mod routes;
mod state;
mod utils;

use std::sync::Arc;

use configs::{database, AppConfig};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env();
    let pool = database::connect(&config).await?;

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
