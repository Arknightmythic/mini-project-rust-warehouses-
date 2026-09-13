use opentelemetry::global;
// Invisible but required: provider.tracer() comes from this trait.
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::env_var_or;

// Must be held for the lifetime of the process. The batch exporter ships spans on
// a background thread, so dropping the provider without shutdown silently discards
// whatever has not been flushed yet. "Why is Jaeger empty" is usually this.
pub struct TelemetryGuard(Option<SdkTracerProvider>);

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.0.take() {
            if let Err(err) = provider.shutdown() {
                eprintln!("failed to shut down tracer provider: {err}");
            }
        }
    }
}

#[must_use = "dropping the guard immediately would stop spans from being exported"]
pub fn init(service_name: &str) -> TelemetryGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,tower_http=debug,sqlx=warn,h2=warn,hyper=warn,tonic=warn")
    });

    let endpoint = env_var_or("OTEL_EXPORTER_OTLP_ENDPOINT", "");

    // No collector configured (plain `cargo run` on the host): log to stdout only.
    // Tracing must never be a startup dependency.
    if endpoint.is_empty() {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();

        tracing::info!(service = service_name, "telemetry initialised (logs only)");
        return TelemetryGuard(None);
    }

    // W3C traceparent. Without this the propagator is a no-op and every service
    // starts its own disconnected trace.
    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.clone())
        .build()
        .expect("failed to build the OTLP span exporter");

    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_string())
                .build(),
        )
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer(service_name.to_string());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    tracing::info!(service = service_name, otlp = %endpoint, "telemetry initialised");
    TelemetryGuard(Some(provider))
}
