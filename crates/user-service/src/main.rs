mod config;
mod models;
mod repositories;
mod service;
mod utils;

use std::sync::Arc;

use tonic::transport::Server;
use wms_core::db::DbConfig;
use wms_proto::user::v1::user_service_server::UserServiceServer;

use config::ServiceConfig;
use service::UserGrpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    wms_core::telemetry::init("user-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    // This service owns its schema and nothing else, so the migrator lives here
    // rather than in the shared crate.
    sqlx::migrate!("./migrations").run(&pool).await?;

    let addr = config.grpc_addr.parse()?;
    let grpc = UserGrpcService::new(pool, Arc::new(config));

    tracing::info!("user-service listening on {addr}");
    Server::builder()
        .add_service(UserServiceServer::new(grpc))
        .serve(addr)
        .await?;

    Ok(())
}
