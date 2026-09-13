use std::future::Future;

use futures_lite::stream::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions};
use lapin::types::FieldTable;
use lapin::{Channel, Connection, ConnectionProperties};
use opentelemetry::global;
use opentelemetry::propagation::Extractor;
use serde::de::DeserializeOwned;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::envelope::Envelope;
use crate::topology;

pub async fn connect(url: &str) -> anyhow::Result<(Connection, Channel)> {
    let connection = Connection::connect(url, ConnectionProperties::default()).await?;
    let channel = connection.create_channel().await?;

    topology::declare(&channel).await?;

    // Take one unacked message at a time. Without this the broker pushes the whole
    // queue at a single consumer and adding more consumers changes nothing.
    channel.basic_qos(1, BasicQosOptions::default()).await?;

    Ok((connection, channel))
}

pub async fn consume<T, F, Fut>(
    channel: &Channel,
    queue: &str,
    consumer_tag: &str,
    handler: F,
) -> anyhow::Result<()>
where
    T: DeserializeOwned,
    F: Fn(Envelope<T>) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut consumer = channel
        .basic_consume(
            queue.into(),
            consumer_tag.into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!(queue, "consumer started");

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;

        let envelope: Envelope<T> = match serde_json::from_slice(&delivery.data) {
            Ok(value) => value,
            Err(err) => {
                // A message we cannot even parse will never parse. Requeueing it
                // would spin forever, so it goes straight to the dead letter queue.
                tracing::error!(error = %err, "message could not be decoded, dead-lettering");
                delivery.nack(nack_no_requeue()).await?;
                continue;
            }
        };

        // The parent has to be attached while the span is being CREATED. Attaching
        // it after the span is entered fails with "span has already been started",
        // which is how a distributed trace silently ends up in pieces.
        let span = tracing::info_span!(
            "consume",
            event_type = %envelope.event_type,
            event_id = %envelope.event_id,
        );
        if let Some(traceparent) = envelope.trace_parent.clone() {
            let carrier = TraceParentCarrier(traceparent);
            let parent =
                global::get_text_map_propagator(|propagator| propagator.extract(&carrier));
            if let Err(err) = span.set_parent(parent) {
                tracing::debug!(error = %err, "could not attach the producer trace context");
            }
        }

        match handler(envelope).instrument(span).await {
            Ok(()) => {
                delivery.ack(BasicAckOptions::default()).await?;
            }
            Err(err) => {
                // requeue: false is load-bearing. A message that fails
                // deterministically and is requeued becomes an infinite hot loop
                // that pins a CPU core and floods the logs. Sending it to the DLQ
                // costs a manual replay; requeueing it costs an incident.
                //
                // The tradeoff, stated plainly: a TRANSIENT failure (database
                // briefly down) also lands in the DLQ and needs replaying by hand.
                // Retry-with-backoff before dead-lettering is the refinement.
                tracing::error!(error = ?err, "handler failed, dead-lettering");
                delivery.nack(nack_no_requeue()).await?;
            }
        }
    }

    Ok(())
}

fn nack_no_requeue() -> BasicNackOptions {
    BasicNackOptions {
        requeue: false,
        ..BasicNackOptions::default()
    }
}

struct TraceParentCarrier(String);

impl Extractor for TraceParentCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        if key.eq_ignore_ascii_case("traceparent") {
            Some(self.0.as_str())
        } else {
            None
        }
    }

    fn keys(&self) -> Vec<&str> {
        vec!["traceparent"]
    }
}
