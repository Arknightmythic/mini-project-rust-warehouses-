use sqlx::PgPool;
use uuid::Uuid;
use wms_core::AppError;

pub struct NewNotification {
    pub recipient_email: String,
    pub subject: String,
    pub body: String,
}

pub async fn already_processed(pool: &PgPool, event_id: Uuid) -> Result<bool, AppError> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT event_id FROM public.processed_events WHERE event_id = $1")
            .bind(event_id)
            .fetch_optional(pool)
            .await?;

    Ok(row.is_some())
}

// The notifications and the dedup marker go in ONE transaction. Writing them
// separately would leave a window where the notification exists but the marker
// does not, and a redelivery would send it twice.
pub async fn record(
    pool: &PgPool,
    event_id: Uuid,
    event_type: &str,
    notifications: &[NewNotification],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    for notification in notifications {
        sqlx::query(
            "INSERT INTO public.notifications (recipient_email, subject, body, event_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&notification.recipient_email)
        .bind(&notification.subject)
        .bind(&notification.body)
        .bind(event_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO public.processed_events (event_id, event_type)
         VALUES ($1, $2)
         ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(event_id)
    .bind(event_type)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}
