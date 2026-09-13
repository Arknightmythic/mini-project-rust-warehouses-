mod config;
mod consumer;
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
    let _telemetry = wms_core::telemetry::init("product-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // A cache that cannot be reached must not stop the service from starting.
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

    // The consumer runs alongside the gRPC server on its own task. If the broker
    // is unreachable the server still starts: stock rollups simply stop updating,
    // which is a degraded read model, not an outage.
    if config.amqp_url.is_empty() {
        tracing::info!("no AMQP_URL set, stock rollups will not be updated");
    } else {
        let rollup = consumer::StockRollupConsumer::new(pool.clone(), cache.clone());
        let amqp_url = config.amqp_url.clone();

        tokio::spawn(async move {
            match wms_events::consumer::connect(&amqp_url).await {
                Ok((_connection, channel)) => {
                    let result = wms_events::consumer::consume(
                        &channel,
                        wms_events::topology::PRODUCT_QUEUE,
                        "product-service",
                        |envelope| rollup.handle(envelope),
                    )
                    .await;

                    if let Err(err) = result {
                        tracing::error!(error = ?err, "stock rollup consumer stopped");
                    }
                }
                Err(err) => tracing::warn!(error = %err, "broker unavailable, no stock rollups"),
            }
        });
    }

    let addr = config.grpc_addr.parse()?;

    tracing::info!("product-service listening on {addr}");
    Server::builder()
        .layer(wms_core::grpc::trace_layer())
        .add_service(ProductServiceServer::new(ProductGrpcService::new(pool, cache)))
        .serve(addr)
        .await?;

    Ok(())
}
