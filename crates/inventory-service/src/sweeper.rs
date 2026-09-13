use std::time::Duration;

use crate::repositories::saga_repository;
use crate::service::InventoryGrpcService;

// A saga has no coordinator watching over it, so something has to notice when a
// step never arrives. Without this, a reservation whose confirmer crashed would
// hold stock hostage forever with no error anywhere to explain why.
pub fn spawn(service: InventoryGrpcService, interval_secs: u64, timeout_secs: i64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));

        loop {
            ticker.tick().await;

            let stale = match saga_repository::find_stale(service.pool(), timeout_secs).await {
                Ok(ids) => ids,
                Err(err) => {
                    tracing::error!(error = ?err, "sweeper could not scan for stale reservations");
                    continue;
                }
            };

            for shipment_id in stale {
                match service
                    .compensate(shipment_id, "reservation expired")
                    .await
                {
                    Ok(()) => tracing::warn!(shipment_id, "stale reservation swept"),
                    // Losing a race with a manual cancel or confirm is normal and
                    // harmless: whoever got there first already resolved it.
                    Err(err) => tracing::debug!(shipment_id, error = ?err, "sweep skipped"),
                }
            }
        }
    });
}
