mod config;
mod models;
mod repositories;
mod service;

use tonic::transport::Server;
use wms_core::db::DbConfig;
use wms_proto::warehouse::v1::warehouse_service_server::WarehouseServiceServer;

use config::ServiceConfig;
use service::WarehouseGrpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    wms_core::telemetry::init("warehouse-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let addr = config.grpc_addr.parse()?;

    tracing::info!("warehouse-service listening on {addr}");
    Server::builder()
        .add_service(WarehouseServiceServer::new(WarehouseGrpcService::new(pool)))
        .serve(addr)
        .await?;

    Ok(())
}
