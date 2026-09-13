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
    let _telemetry = wms_core::telemetry::init("warehouse-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let cache = if config.redis_url.is_empty() {
        tracing::info!("no REDIS_URL set, running without a cache");
        None
    } else {
        match wms_core::cache::Cache::connect(&config.redis_url, config.cache_ttl_secs).await {
            Ok(cache) => {
                tracing::info!(ttl = config.cache_ttl_secs, "cache connected");
                Some(cache)
            }
            Err(err) => {
                tracing::warn!(error = %err, "cache unavailable, serving from the database only");
                None
            }
        }
    };

    let addr = config.grpc_addr.parse()?;

    tracing::info!("warehouse-service listening on {addr}");
    Server::builder()
        .layer(wms_core::grpc::trace_layer())
        .add_service(WarehouseServiceServer::new(WarehouseGrpcService::new(pool, cache)))
        .serve(addr)
        .await?;

    Ok(())
}
