mod config;
mod models;
mod repositories;
mod service;

use tonic::transport::Server;
use wms_core::db::DbConfig;
use wms_proto::product::v1::product_service_server::ProductServiceServer;

use config::ServiceConfig;
use service::ProductGrpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    wms_core::telemetry::init("product-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let addr = config.grpc_addr.parse()?;

    tracing::info!("product-service listening on {addr}");
    Server::builder()
        .add_service(ProductServiceServer::new(ProductGrpcService::new(pool)))
        .serve(addr)
        .await?;

    Ok(())
}
