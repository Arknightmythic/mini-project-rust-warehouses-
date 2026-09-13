use sqlx::PgPool;
use wms_core::AppError;

use crate::models::product::Product;

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<Product>, AppError> {
    Ok(
        sqlx::query_as::<_, Product>("SELECT * FROM public.products WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

// One query for N ids. The alternative is N round trips across the network, which
// is the difference between a fast receipt and a slow one once inventory calls it.
pub async fn find_by_ids(pool: &PgPool, ids: &[i64]) -> Result<Vec<Product>, AppError> {
    Ok(
        sqlx::query_as::<_, Product>("SELECT * FROM public.products WHERE id = ANY($1) ORDER BY id")
            .bind(ids)
            .fetch_all(pool)
            .await?,
    )
}

pub async fn list(pool: &PgPool, include_inactive: bool) -> Result<Vec<Product>, AppError> {
    let sql = if include_inactive {
        "SELECT * FROM public.products ORDER BY id"
    } else {
        "SELECT * FROM public.products WHERE is_active = TRUE ORDER BY id"
    };

    Ok(sqlx::query_as::<_, Product>(sql).fetch_all(pool).await?)
}

pub async fn create(
    pool: &PgPool,
    sku: &str,
    name: &str,
    description: Option<&str>,
    unit: &str,
    barcode: Option<&str>,
    category_id: Option<i64>,
) -> Result<Product, AppError> {
    Ok(sqlx::query_as::<_, Product>(
        "INSERT INTO public.products (sku, name, description, unit, barcode, category_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING *",
    )
    .bind(sku)
    .bind(name)
    .bind(description)
    .bind(unit)
    .bind(barcode)
    .bind(category_id)
    .fetch_one(pool)
    .await?)
}

pub async fn update(
    pool: &PgPool,
    id: i64,
    name: Option<&str>,
    description: Option<&str>,
    unit: Option<&str>,
    barcode: Option<&str>,
    category_id: Option<i64>,
) -> Result<Option<Product>, AppError> {
    Ok(sqlx::query_as::<_, Product>(
        "UPDATE public.products SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            unit = COALESCE($4, unit),
            barcode = COALESCE($5, barcode),
            category_id = COALESCE($6, category_id),
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(unit)
    .bind(barcode)
    .bind(category_id)
    .fetch_optional(pool)
    .await?)
}

// Master data is never hard deleted. Rows in other services reference these ids
// and there is no foreign key to stop a delete from orphaning them.
pub async fn deactivate(pool: &PgPool, id: i64) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE public.products SET is_active = FALSE, updated_at = CURRENT_TIMESTAMP
         WHERE id = $1 AND is_active = TRUE",
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

// Race-safe by construction, same rule as the stock balances themselves: the
// read and the write are one statement, never a SELECT followed by an UPDATE.
pub async fn add_to_stock_rollup(
    pool: &PgPool,
    product_id: i64,
    quantity: i64,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE public.products
            SET total_stock_cached = total_stock_cached + $2,
                stock_synced_at = CURRENT_TIMESTAMP
          WHERE id = $1",
    )
    .bind(product_id)
    .bind(quantity)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
