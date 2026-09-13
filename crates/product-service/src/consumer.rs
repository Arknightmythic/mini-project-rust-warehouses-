use sqlx::PgPool;
use wms_core::cache::Cache;
use wms_events::{Envelope, StockReceived};

const STOCK_RECEIVED: &str = "inventory.stock.received";

use crate::repositories::product_repository;

// product-service is both a gRPC server AND an event consumer. Being event-driven
// on one edge says nothing about the other: services are not "sync" or "async",
// they use whichever fits each interaction.
pub struct StockRollupConsumer {
    pool: PgPool,
    cache: Option<Cache>,
}

impl StockRollupConsumer {
    pub fn new(pool: PgPool, cache: Option<Cache>) -> Self {
        Self { pool, cache }
    }

    pub async fn handle(&self, envelope: Envelope<serde_json::Value>) -> anyhow::Result<()> {
        // Shipments move stock too, but this rollup only tracks receipts for now.
        // Ignoring the rest is a decision, not an oversight.
        if envelope.event_type != STOCK_RECEIVED {
            tracing::debug!(event_type = %envelope.event_type, "event type not handled, skipping");
            return Ok(());
        }

        let payload: StockReceived = envelope.payload_as()?;

        for line in &payload.lines {
            let updated =
                product_repository::add_to_stock_rollup(&self.pool, line.product_id, line.quantity)
                    .await?;

            if updated == 0 {
                // The product id came from another service and there is no foreign
                // key to stop it pointing at nothing. Skipping is right: failing
                // would dead-letter an event that will never succeed.
                tracing::warn!(
                    product_id = line.product_id,
                    "stock event references an unknown product, skipping"
                );
                continue;
            }

            // The rollup just changed, so the cached copy is now wrong. This is the
            // cross-service half of invalidation: product-service only learned that
            // stock moved because an event told it.
            if let Some(cache) = &self.cache {
                cache.invalidate(&format!("product:{}", line.product_id)).await;
            }
        }

        tracing::info!(
            event_id = %envelope.event_id,
            receipt_id = payload.receipt_id,
            lines = payload.lines.len(),
            "stock rollup updated"
        );

        Ok(())
    }
}
