mod config;
mod handler;
mod repositories;

use tonic::transport::Endpoint;
use wms_core::db::DbConfig;
use wms_core::grpc::TraceInterceptor;
use wms_proto::user::v1::user_service_client::UserServiceClient;

use config::ServiceConfig;
use handler::Handler;

// This service has no HTTP surface and no gRPC server. It exists only to react to
// things other services did, which is the whole point of including it: not every
// service is request/response.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));
    let _telemetry = wms_core::telemetry::init("notification-service");

    let config = ServiceConfig::from_env();
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let inject: TraceInterceptor = wms_core::grpc::inject_trace_context;
    let users = UserServiceClient::with_interceptor(
        Endpoint::from_shared(config.user_service_url.clone())?.connect_lazy(),
        inject,
    );

    let handler = Handler::new(pool, users);

    // Unlike the cache and the publisher, the broker IS a hard dependency here:
    // a consumer with nothing to consume from has no reason to be running.
    let (_connection, channel) = wms_events::consumer::connect(&config.amqp_url).await?;

    tracing::info!("notification-service waiting for events");

    wms_events::consumer::consume(
        &channel,
        wms_events::topology::NOTIFICATION_QUEUE,
        "notification-service",
        |envelope| {
            let handler = handler.clone();
            async move { handler.handle(envelope).await }
        },
    )
    .await?;

    Ok(())
}
