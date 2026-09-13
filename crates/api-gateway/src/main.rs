mod config;
mod dto;
mod middlewares;
mod routes;
mod state;

use std::sync::Arc;

use tonic::transport::Endpoint;
use wms_core::grpc::TraceInterceptor;
use wms_proto::user::v1::user_service_client::UserServiceClient;
use wms_proto::inventory::v1::inventory_service_client::InventoryServiceClient;
use wms_proto::product::v1::product_service_client::ProductServiceClient;
use wms_proto::warehouse::v1::warehouse_service_client::WarehouseServiceClient;

use config::GatewayConfig;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    let _telemetry = wms_core::telemetry::init("api-gateway");

    let config = GatewayConfig::from_env();

    // connect_lazy means the gateway boots even when a downstream is down; the
    // failure then surfaces per request as 503 instead of blocking startup.
    // with_interceptor, not new: every outgoing call carries the caller's
    // traceparent automatically. Trace context is ambient, so hiding it is right.
    let inject: TraceInterceptor = wms_core::grpc::inject_trace_context;

    let user_channel = Endpoint::from_shared(config.user_service_url.clone())?.connect_lazy();
    let users = UserServiceClient::with_interceptor(user_channel, inject);

    let warehouse_channel =
        Endpoint::from_shared(config.warehouse_service_url.clone())?.connect_lazy();
    let warehouses = WarehouseServiceClient::with_interceptor(warehouse_channel, inject);

    let product_channel = Endpoint::from_shared(config.product_service_url.clone())?.connect_lazy();
    let products = ProductServiceClient::with_interceptor(product_channel, inject);

    let inventory_channel =
        Endpoint::from_shared(config.inventory_service_url.clone())?.connect_lazy();
    let inventory = InventoryServiceClient::with_interceptor(inventory_channel, inject);

    let addr = format!("{}:{}", config.server_url, config.server_port);

    let state = AppState {
        users,
        warehouses,
        products,
        inventory,
        config: Arc::new(config),
    };

    let app = routes::build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("api-gateway listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
