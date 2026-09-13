use sqlx::PgPool;

use crate::models::warehouse::Warehouse;

pub async fn list(pool: &PgPool) -> Result<Vec<Warehouse>, sqlx::Error> {
    sqlx::query_as::<_, Warehouse>(
        "SELECT * FROM public.warehouses WHERE delete_at IS NULL ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<Warehouse>, sqlx::Error> {
    sqlx::query_as::<_, Warehouse>(
        "SELECT * FROM public.warehouses WHERE id = $1 AND delete_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn create(
    pool: &PgPool,
    name: &str,
    address: &str,
    phone: Option<&str>,
    photo: Option<&str>,
) -> Result<Warehouse, sqlx::Error> {
    sqlx::query_as::<_, Warehouse>(
        "INSERT INTO public.warehouses (name, address, phone, photo)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(name)
    .bind(address)
    .bind(phone)
    .bind(photo)
    .fetch_one(pool)
    .await
}

pub async fn update(
    pool: &PgPool,
    id: i64,
    name: Option<&str>,
    address: Option<&str>,
    phone: Option<&str>,
    photo: Option<&str>,
) -> Result<Option<Warehouse>, sqlx::Error> {
    sqlx::query_as::<_, Warehouse>(
        "UPDATE public.warehouses SET
            name = COALESCE($2, name),
            address = COALESCE($3, address),
            phone = COALESCE($4, phone),
            photo = COALESCE($5, photo),
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $1 AND delete_at IS NULL
         RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(address)
    .bind(phone)
    .bind(photo)
    .fetch_optional(pool)
    .await
}

pub async fn soft_delete(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE public.warehouses SET delete_at = CURRENT_TIMESTAMP
         WHERE id = $1 AND delete_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
