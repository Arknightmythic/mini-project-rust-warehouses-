mod config;
mod models;
mod repositories;
mod service;

use tonic::transport::{Endpoint, Server};
use wms_core::db::DbConfig;
use wms_proto::inventory::v1::inventory_service_server::InventoryServiceServer;
use wms_proto::product::v1::product_service_client::ProductServiceClient;
use wms_proto::warehouse::v1::warehouse_service_client::WarehouseServiceClient;

use config::ServiceConfig;
use service::InventoryGrpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    wms_core::telemetry::init("inventory-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // Lazy channels: this service starts even when a dependency is down, and the
    // failure surfaces per request instead of blocking boot.
    let warehouses = WarehouseServiceClient::new(
        Endpoint::from_shared(config.warehouse_service_url.clone())?.connect_lazy(),
    );
    let products = ProductServiceClient::new(
        Endpoint::from_shared(config.product_service_url.clone())?.connect_lazy(),
    );

    let addr = config.grpc_addr.parse()?;

    tracing::info!("inventory-service listening on {addr}");
    Server::builder()
        .add_service(InventoryServiceServer::new(InventoryGrpcService::new(
            pool, warehouses, products,
        )))
        .serve(addr)
        .await?;

    Ok(())
}
