mod config;
mod dto;
mod middlewares;
mod routes;
mod state;

use std::sync::Arc;

use tonic::transport::Endpoint;
use wms_proto::user::v1::user_service_client::UserServiceClient;

use config::GatewayConfig;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    wms_core::telemetry::init("api-gateway");

    let config = GatewayConfig::from_env();

    // connect_lazy means the gateway boots even when a downstream is down; the
    // failure then surfaces per request as 503 instead of blocking startup.
    let channel = Endpoint::from_shared(config.user_service_url.clone())?.connect_lazy();
    let users = UserServiceClient::new(channel);

    let addr = format!("{}:{}", config.server_url, config.server_port);

    let state = AppState {
        users,
        config: Arc::new(config),
    };

    let app = routes::build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("api-gateway listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
