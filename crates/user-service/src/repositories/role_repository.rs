use sqlx::PgPool;
use wms_core::AppError;

use crate::models::role::Role;

pub async fn list(pool: &PgPool) -> Result<Vec<Role>, AppError> {
    Ok(sqlx::query_as::<_, Role>("SELECT * FROM public.roles ORDER BY id")
        .fetch_all(pool)
        .await?)
}

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<Role>, AppError> {
    Ok(
        sqlx::query_as::<_, Role>("SELECT * FROM public.roles WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn create(pool: &PgPool, name: &str) -> Result<Role, AppError> {
    Ok(
        sqlx::query_as::<_, Role>("INSERT INTO public.roles (name) VALUES ($1) RETURNING *")
            .bind(name)
            .fetch_one(pool)
            .await?,
    )
}

pub async fn update(pool: &PgPool, id: i64, name: &str) -> Result<Option<Role>, AppError> {
    Ok(sqlx::query_as::<_, Role>(
        "UPDATE public.roles SET name = $2, updated_at = CURRENT_TIMESTAMP
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(name)
    .fetch_optional(pool)
    .await?)
}

pub async fn delete(pool: &PgPool, id: i64) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM public.roles WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
