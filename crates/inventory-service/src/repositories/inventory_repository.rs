use sqlx::PgPool;
use wms_core::AppError;

use crate::models::stock::{InboundReceipt, InboundReceiptItem, StockBalance, StockMovement};

pub struct NewReceiptItem {
    pub product_id: i64,
    pub quantity: i64,
    pub unit_cost: i64,
}

pub async fn find_receipt_by_idempotency_key(
    pool: &PgPool,
    key: &str,
) -> Result<Option<InboundReceipt>, AppError> {
    Ok(sqlx::query_as::<_, InboundReceipt>(
        "SELECT id, warehouse_id, reference_no, created_at
         FROM public.inbound_receipts
         WHERE idempotency_key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?)
}

pub async fn list_receipt_items(
    pool: &PgPool,
    receipt_id: i64,
) -> Result<Vec<InboundReceiptItem>, AppError> {
    Ok(sqlx::query_as::<_, InboundReceiptItem>(
        "SELECT product_id, quantity, unit_cost
         FROM public.inbound_receipt_items
         WHERE receipt_id = $1
         ORDER BY id",
    )
    .bind(receipt_id)
    .fetch_all(pool)
    .await?)
}

// Everything here is one transaction: the receipt, its lines, the ledger entries
// and the balance updates either all land or none do.
pub async fn insert_receipt(
    pool: &PgPool,
    warehouse_id: i64,
    reference_no: Option<&str>,
    idempotency_key: &str,
    created_by: i64,
    items: &[NewReceiptItem],
    outbox_event: Option<&crate::outbox::OutboxEvent>,
) -> Result<InboundReceipt, AppError> {
    let mut tx = pool.begin().await?;

    let receipt = sqlx::query_as::<_, InboundReceipt>(
        "INSERT INTO public.inbound_receipts (warehouse_id, reference_no, idempotency_key, created_by)
         VALUES ($1, $2, $3, $4)
         RETURNING id, warehouse_id, reference_no, created_at",
    )
    .bind(warehouse_id)
    .bind(reference_no)
    .bind(idempotency_key)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    for item in items {
        sqlx::query(
            "INSERT INTO public.inbound_receipt_items (receipt_id, product_id, quantity, unit_cost)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(receipt.id)
        .bind(item.product_id)
        .bind(item.quantity)
        .bind(item.unit_cost)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO public.stock_movements
                (warehouse_id, product_id, movement_type, quantity, reference_type, reference_id)
             VALUES ($1, $2, 'IN', $3, 'INBOUND_RECEIPT', $4)",
        )
        .bind(warehouse_id)
        .bind(item.product_id)
        .bind(item.quantity)
        .bind(receipt.id)
        .execute(&mut *tx)
        .await?;

        // One statement does both the check and the write. Never SELECT then
        // UPDATE from Rust: two concurrent receipts would silently lose an
        // increment, and it would almost never reproduce in testing.
        sqlx::query(
            "INSERT INTO public.stock_balances (warehouse_id, product_id, qty_on_hand)
             VALUES ($1, $2, $3)
             ON CONFLICT (warehouse_id, product_id)
             DO UPDATE SET
                qty_on_hand = stock_balances.qty_on_hand + EXCLUDED.qty_on_hand,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(warehouse_id)
        .bind(item.product_id)
        .bind(item.quantity)
        .execute(&mut *tx)
        .await?;
    }

    // Same transaction as everything above. Either the stock moved and the event
    // exists, or neither happened.
    if let Some(event) = outbox_event {
        // The payload was built before the row existed, so the real id goes in now.
        let mut payload = event.payload.clone();
        if let Some(inner) = payload.get_mut("payload").and_then(|v| v.as_object_mut()) {
            inner.insert("receipt_id".to_string(), serde_json::json!(receipt.id));
        }

        crate::outbox::enqueue(&mut tx, event, &payload).await?;
    }

    tx.commit().await?;

    // Simulates the process dying in the gap that used to lose events. With the
    // old direct-publish path this kills the event forever; with the outbox the
    // relay picks it up after restart.
    // is_ok() alone is a trap here: docker compose writes ${VAR:-} as an EMPTY
    // variable rather than an absent one, and env::var returns Ok("") for that.
    // The flag has to be checked for a truthy VALUE, not merely for existence.
    if std::env::var("CRASH_AFTER_COMMIT")
        .is_ok_and(|value| !value.is_empty() && value != "0" && value != "false")
    {
        tracing::error!("CRASH_AFTER_COMMIT set, exiting immediately after commit");
        std::process::exit(1);
    }

    Ok(receipt)
}

pub async fn find_balance(
    pool: &PgPool,
    warehouse_id: i64,
    product_id: i64,
) -> Result<Option<StockBalance>, AppError> {
    Ok(sqlx::query_as::<_, StockBalance>(
        "SELECT warehouse_id, product_id, qty_on_hand, qty_reserved, updated_at
         FROM public.stock_balances
         WHERE warehouse_id = $1 AND product_id = $2",
    )
    .bind(warehouse_id)
    .bind(product_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn list_balances(
    pool: &PgPool,
    warehouse_id: Option<i64>,
) -> Result<Vec<StockBalance>, AppError> {
    Ok(sqlx::query_as::<_, StockBalance>(
        "SELECT warehouse_id, product_id, qty_on_hand, qty_reserved, updated_at
         FROM public.stock_balances
         WHERE ($1::BIGINT IS NULL OR warehouse_id = $1)
         ORDER BY warehouse_id, product_id",
    )
    .bind(warehouse_id)
    .fetch_all(pool)
    .await?)
}

pub async fn list_movements(
    pool: &PgPool,
    warehouse_id: Option<i64>,
    product_id: Option<i64>,
) -> Result<Vec<StockMovement>, AppError> {
    Ok(sqlx::query_as::<_, StockMovement>(
        "SELECT * FROM public.stock_movements
         WHERE ($1::BIGINT IS NULL OR warehouse_id = $1)
           AND ($2::BIGINT IS NULL OR product_id = $2)
         ORDER BY id DESC",
    )
    .bind(warehouse_id)
    .bind(product_id)
    .fetch_all(pool)
    .await?)
}
