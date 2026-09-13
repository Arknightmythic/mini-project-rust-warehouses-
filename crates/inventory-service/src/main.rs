mod config;
mod models;
mod repositories;
mod saga;
mod sweeper;
mod service;

use tonic::transport::{Endpoint, Server};
use wms_core::db::DbConfig;
use wms_core::grpc::TraceInterceptor;
use wms_proto::inventory::v1::inventory_service_server::InventoryServiceServer;
use wms_proto::product::v1::product_service_client::ProductServiceClient;
use wms_proto::warehouse::v1::warehouse_service_client::WarehouseServiceClient;

use config::ServiceConfig;
use service::InventoryGrpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    let _telemetry = wms_core::telemetry::init("inventory-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // Lazy channels: this service starts even when a dependency is down, and the
    // failure surfaces per request instead of blocking boot.
    let inject: TraceInterceptor = wms_core::grpc::inject_trace_context;

    let warehouses = WarehouseServiceClient::with_interceptor(
        Endpoint::from_shared(config.warehouse_service_url.clone())?.connect_lazy(),
        inject,
    );
    let products = ProductServiceClient::with_interceptor(
        Endpoint::from_shared(config.product_service_url.clone())?.connect_lazy(),
        inject,
    );

    // Like the cache: a broker that cannot be reached must not stop the service
    // from starting. Receipts still work; they just go unannounced.
    let events = if config.amqp_url.is_empty() {
        tracing::info!("no AMQP_URL set, running without event publishing");
        None
    } else {
        match wms_events::EventPublisher::connect(&config.amqp_url).await {
            Ok(publisher) => {
                tracing::info!("event publisher connected");
                Some(publisher)
            }
            Err(err) => {
                tracing::warn!(error = %err, "broker unavailable, events will not be published");
                None
            }
        }
    };

    let addr = config.grpc_addr.parse()?;

    let config = std::sync::Arc::new(config);
    let service = InventoryGrpcService::new(pool, warehouses, products, events, config.clone());

    sweeper::spawn(
        service.clone(),
        config.sweeper_interval_secs,
        config.reservation_timeout_secs,
    );

    tracing::info!(
        sweep_every = config.sweeper_interval_secs,
        expire_after = config.reservation_timeout_secs,
        "inventory-service listening on {addr}"
    );
    Server::builder()
        .layer(wms_core::grpc::trace_layer())
        .add_service(InventoryServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
