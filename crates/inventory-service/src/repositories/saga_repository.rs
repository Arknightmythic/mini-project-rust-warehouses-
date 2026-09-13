use sqlx::PgPool;
use wms_core::AppError;

use crate::models::stock::{OutboundShipment, ShipmentLine};

pub struct NewShipmentLine {
    pub product_id: i64,
    pub quantity: i64,
}

pub async fn find_by_idempotency_key(
    pool: &PgPool,
    key: &str,
) -> Result<Option<OutboundShipment>, AppError> {
    Ok(sqlx::query_as::<_, OutboundShipment>(
        "SELECT id, warehouse_id, reference_no, status, created_at, updated_at
         FROM public.outbound_shipments
         WHERE idempotency_key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?)
}

pub async fn find(pool: &PgPool, id: i64) -> Result<Option<OutboundShipment>, AppError> {
    Ok(sqlx::query_as::<_, OutboundShipment>(
        "SELECT id, warehouse_id, reference_no, status, created_at, updated_at
         FROM public.outbound_shipments WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn list_lines(pool: &PgPool, shipment_id: i64) -> Result<Vec<ShipmentLine>, AppError> {
    Ok(sqlx::query_as::<_, ShipmentLine>(
        "SELECT product_id, quantity FROM public.outbound_shipment_items
         WHERE shipment_id = $1 ORDER BY id",
    )
    .bind(shipment_id)
    .fetch_all(pool)
    .await?)
}

// STEP 1 - RESERVE.
//
// Inside one service with one database, a plain transaction still solves
// everything. Do not reach for a saga when a transaction will do.
//
// The conditional UPDATE is the check AND the write in one statement. A SELECT to
// verify availability followed by an UPDATE would let two concurrent shipments
// both pass the check and oversell the warehouse.
pub async fn reserve(
    pool: &PgPool,
    warehouse_id: i64,
    reference_no: Option<&str>,
    idempotency_key: &str,
    created_by: i64,
    lines: &[NewShipmentLine],
) -> Result<OutboundShipment, AppError> {
    let mut tx = pool.begin().await?;

    let shipment = sqlx::query_as::<_, OutboundShipment>(
        "INSERT INTO public.outbound_shipments
            (warehouse_id, reference_no, idempotency_key, created_by, status)
         VALUES ($1, $2, $3, $4, 'RESERVED')
         RETURNING id, warehouse_id, reference_no, status, created_at, updated_at",
    )
    .bind(warehouse_id)
    .bind(reference_no)
    .bind(idempotency_key)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    for line in lines {
        sqlx::query(
            "INSERT INTO public.outbound_shipment_items (shipment_id, product_id, quantity)
             VALUES ($1, $2, $3)",
        )
        .bind(shipment.id)
        .bind(line.product_id)
        .bind(line.quantity)
        .execute(&mut *tx)
        .await?;

        let result = sqlx::query(
            "UPDATE public.stock_balances
                SET qty_reserved = qty_reserved + $3,
                    updated_at = CURRENT_TIMESTAMP
              WHERE warehouse_id = $1
                AND product_id = $2
                AND (qty_on_hand - qty_reserved) >= $3",
        )
        .bind(warehouse_id)
        .bind(line.product_id)
        .bind(line.quantity)
        .execute(&mut *tx)
        .await?;

        // rows_affected == 0 means the WHERE clause rejected it: no balance row, or
        // not enough available. Rolling the whole transaction back is what makes a
        // half-reserved shipment impossible.
        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(AppError::Conflict(format!(
                "insufficient stock for product {}",
                line.product_id
            )));
        }
    }

    tx.commit().await?;

    Ok(shipment)
}

// STEP 2 - CONFIRM. The reserved quantity finally leaves the warehouse.
pub async fn confirm(pool: &PgPool, shipment_id: i64) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;

    // Claiming the row by status is what makes two concurrent confirmations safe:
    // only one UPDATE can match status = RESERVED.
    let claimed = sqlx::query(
        "UPDATE public.outbound_shipments
            SET status = 'SHIPPED', updated_at = CURRENT_TIMESTAMP
          WHERE id = $1 AND status = 'RESERVED'",
    )
    .bind(shipment_id)
    .execute(&mut *tx)
    .await?;

    if claimed.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    let warehouse: (i64,) =
        sqlx::query_as("SELECT warehouse_id FROM public.outbound_shipments WHERE id = $1")
            .bind(shipment_id)
            .fetch_one(&mut *tx)
            .await?;

    let lines = sqlx::query_as::<_, ShipmentLine>(
        "SELECT product_id, quantity FROM public.outbound_shipment_items WHERE shipment_id = $1",
    )
    .bind(shipment_id)
    .fetch_all(&mut *tx)
    .await?;

    for line in &lines {
        sqlx::query(
            "UPDATE public.stock_balances
                SET qty_on_hand = qty_on_hand - $3,
                    qty_reserved = qty_reserved - $3,
                    updated_at = CURRENT_TIMESTAMP
              WHERE warehouse_id = $1 AND product_id = $2",
        )
        .bind(warehouse.0)
        .bind(line.product_id)
        .bind(line.quantity)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO public.stock_movements
                (warehouse_id, product_id, movement_type, quantity, reference_type, reference_id)
             VALUES ($1, $2, 'OUT', $3, 'OUTBOUND_SHIPMENT', $4)",
        )
        .bind(warehouse.0)
        .bind(line.product_id)
        .bind(line.quantity)
        .bind(shipment_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(true)
}

// STEP 3 - COMPENSATE.
//
// There is no distributed rollback. This is not an undo: it is another forward
// write, recorded as its own fact, that happens to release what step 1 set aside.
pub async fn release(pool: &PgPool, shipment_id: i64) -> Result<bool, AppError> {
    let mut tx = pool.begin().await?;

    let claimed = sqlx::query(
        "UPDATE public.outbound_shipments
            SET status = 'CANCELLED', updated_at = CURRENT_TIMESTAMP
          WHERE id = $1 AND status = 'RESERVED'",
    )
    .bind(shipment_id)
    .execute(&mut *tx)
    .await?;

    if claimed.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    let warehouse: (i64,) =
        sqlx::query_as("SELECT warehouse_id FROM public.outbound_shipments WHERE id = $1")
            .bind(shipment_id)
            .fetch_one(&mut *tx)
            .await?;

    let lines = sqlx::query_as::<_, ShipmentLine>(
        "SELECT product_id, quantity FROM public.outbound_shipment_items WHERE shipment_id = $1",
    )
    .bind(shipment_id)
    .fetch_all(&mut *tx)
    .await?;

    for line in &lines {
        sqlx::query(
            "UPDATE public.stock_balances
                SET qty_reserved = qty_reserved - $3,
                    updated_at = CURRENT_TIMESTAMP
              WHERE warehouse_id = $1 AND product_id = $2",
        )
        .bind(warehouse.0)
        .bind(line.product_id)
        .bind(line.quantity)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(true)
}

// What the sweeper hunts for: reservations nobody ever resolved either way,
// usually because the confirming caller crashed or a downstream went away.
pub async fn find_stale(pool: &PgPool, older_than_secs: i64) -> Result<Vec<i64>, AppError> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM public.outbound_shipments
          WHERE status = 'RESERVED'
            AND created_at < CURRENT_TIMESTAMP - make_interval(secs => $1)
          ORDER BY id",
    )
    .bind(older_than_secs as f64)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}
