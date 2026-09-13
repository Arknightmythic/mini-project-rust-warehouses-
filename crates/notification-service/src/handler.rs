use sqlx::PgPool;
use tonic::Request;
use wms_core::AppError;
use wms_core::grpc::TracedChannel;
use wms_events::{Envelope, StockReceived};

const STOCK_RECEIVED: &str = "inventory.stock.received";
use wms_proto::user::v1 as user_v1;
use wms_proto::user::v1::user_service_client::UserServiceClient;

use crate::repositories::notification_repository::{self, NewNotification};

const NOTIFY_ROLE: &str = "warehouse_manager";

#[derive(Clone)]
pub struct Handler {
    pool: PgPool,
    users: UserServiceClient<TracedChannel>,
}

impl Handler {
    pub fn new(pool: PgPool, users: UserServiceClient<TracedChannel>) -> Self {
        Self { pool, users }
    }

    // The queue binds inventory.stock.# so it receives every stock event. Types
    // this service does not care about are ACKED and ignored: dead-lettering a
    // valid message just because we have no use for it would be wrong.
    pub async fn handle(&self, envelope: Envelope<serde_json::Value>) -> anyhow::Result<()> {
        if envelope.event_type != STOCK_RECEIVED {
            tracing::debug!(event_type = %envelope.event_type, "event type not handled, skipping");
            return Ok(());
        }

        self.handle_stock_received(envelope).await
    }

    async fn handle_stock_received(
        &self,
        envelope: Envelope<serde_json::Value>,
    ) -> anyhow::Result<()> {
        // Deduplicate first. The broker redelivers when an ack is lost, so this
        // handler will see the same event twice sooner or later.
        if notification_repository::already_processed(&self.pool, envelope.event_id).await? {
            tracing::info!(event_id = %envelope.event_id, "event already processed, skipping");
            return Ok(());
        }

        let payload: StockReceived = envelope.payload_as()?;
        let payload = &payload;

        // A consumer is still a service: it makes synchronous calls of its own.
        // Being event-driven on the way in says nothing about the way out.
        let mut users = self.users.clone();
        let recipients = users
            .list_users_by_role(Request::new(user_v1::ListUsersByRoleRequest {
                role: NOTIFY_ROLE.to_string(),
            }))
            .await
            .map_err(AppError::from)?
            .into_inner()
            .users;

        let total: i64 = payload.lines.iter().map(|line| line.quantity).sum();
        let subject = format!("Stock received at warehouse {}", payload.warehouse_id);
        let body = format!(
            "Receipt #{} recorded {} unit(s) across {} line(s){}.",
            payload.receipt_id,
            total,
            payload.lines.len(),
            payload
                .reference_no
                .as_ref()
                .map(|reference| format!(" for {reference}"))
                .unwrap_or_default(),
        );

        let notifications: Vec<NewNotification> = recipients
            .iter()
            .map(|user| NewNotification {
                recipient_email: user.email.clone(),
                subject: subject.clone(),
                body: body.clone(),
            })
            .collect();

        notification_repository::record(
            &self.pool,
            envelope.event_id,
            &envelope.event_type,
            &notifications,
        )
        .await?;

        // Stub on purpose: actually sending mail is not what this phase is about.
        tracing::info!(
            event_id = %envelope.event_id,
            receipt_id = payload.receipt_id,
            recipients = notifications.len(),
            "notifications queued for delivery"
        );

        Ok(())
    }
}
