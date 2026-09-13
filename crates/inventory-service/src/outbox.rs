use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_core::AppError;
use wms_events::EventPublisher;

// What gets written alongside the domain change, in the same transaction.
pub struct OutboxEvent {
    pub event_id: Uuid,
    pub event_type: String,
    pub routing_key: String,
    pub payload: serde_json::Value,
    pub trace_parent: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PendingRow {
    id: i64,
    routing_key: String,
    payload: serde_json::Value,
    trace_parent: Option<String>,
}

// Takes the caller's transaction rather than a pool. That is the entire point: if
// this row is not written by the same transaction that moved the stock, nothing
// has been solved.
pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    event: &OutboxEvent,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO public.outbox (event_id, event_type, routing_key, payload, trace_parent)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(event.event_id)
    .bind(&event.event_type)
    .bind(&event.routing_key)
    .bind(payload)
    .bind(&event.trace_parent)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

// Drains the outbox to the broker.
//
// Note the ordering: publish, THEN mark sent. A crash in between republishes the
// event on the next pass. That is deliberate - the outbox converts "might be lost"
// into "might be duplicated", and duplicates were already handled by the
// processed_events table on the consumer side. Marking sent first would trade a
// safe problem for an unsafe one.
pub fn spawn_relay(pool: PgPool, publisher: EventPublisher, interval_ms: u64) {
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_millis(interval_ms.max(50)));

        loop {
            ticker.tick().await;

            let pending: Vec<PendingRow> = match sqlx::query_as(
                "SELECT id, routing_key, payload, trace_parent
                   FROM public.outbox
                  WHERE sent_at IS NULL
                  ORDER BY id
                  LIMIT 100",
            )
            .fetch_all(&pool)
            .await
            {
                Ok(rows) => rows,
                Err(err) => {
                    tracing::error!(error = ?err, "outbox relay could not read pending events");
                    continue;
                }
            };

            for row in pending {
                if let Err(err) = publisher
                    .publish_raw(&row.routing_key, &row.payload, row.trace_parent.as_deref())
                    .await
                {
                    // Leave it unsent and try again next tick. The broker being
                    // down delays events; it no longer loses them.
                    tracing::warn!(id = row.id, error = ?err, "outbox publish failed, will retry");
                    break;
                }

                if let Err(err) =
                    sqlx::query("UPDATE public.outbox SET sent_at = CURRENT_TIMESTAMP WHERE id = $1")
                        .bind(row.id)
                        .execute(&pool)
                        .await
                {
                    tracing::error!(id = row.id, error = ?err, "could not mark outbox row sent");
                }
            }
        }
    });
}
