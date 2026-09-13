use std::collections::HashMap;
use std::sync::Arc;

use lapin::options::BasicPublishOptions;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties};
use opentelemetry::global;
use opentelemetry::propagation::Injector;
use serde::Serialize;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::envelope::Envelope;
use crate::topology;

#[derive(Clone)]
pub struct EventPublisher {
    // Held so the connection outlives the channel. Dropping it would silently
    // tear the channel down underneath us.
    _connection: Arc<Connection>,
    channel: Channel,
}

impl EventPublisher {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let connection = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = connection.create_channel().await?;

        topology::declare(&channel).await?;

        Ok(Self {
            _connection: Arc::new(connection),
            channel,
        })
    }

    pub async fn publish<T: Serialize>(
        &self,
        routing_key: &str,
        mut envelope: Envelope<T>,
    ) -> anyhow::Result<()> {
        envelope.trace_parent = current_traceparent();

        let body = serde_json::to_vec(&envelope)?;

        let confirm = self
            .channel
            .basic_publish(
                topology::EXCHANGE.into(),
                routing_key.into(),
                BasicPublishOptions::default(),
                &body,
                // delivery_mode 2 = persistent. Without it the broker keeps the
                // message in memory only and a restart loses it, which defeats the
                // point of a durable queue.
                BasicProperties::default()
                    .with_delivery_mode(2)
                    .with_content_type("application/json".into()),
            )
            .await?;

        // Second await waits for the broker to confirm it took the message.
        // Skipping it means "published" only ever meant "handed to a socket".
        confirm.await?;

        tracing::info!(routing_key, event_id = %envelope.event_id, "event published");
        Ok(())
    }
}

struct MapInjector<'a>(&'a mut HashMap<String, String>);

impl Injector for MapInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

fn current_traceparent() -> Option<String> {
    let context = tracing::Span::current().context();
    let mut carrier = HashMap::new();

    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut MapInjector(&mut carrier))
    });

    carrier.remove("traceparent")
}
