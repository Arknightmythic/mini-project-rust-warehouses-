use sqlx::PgPool;
use wms_core::AppError;

use crate::models::category::Category;

pub async fn list(pool: &PgPool) -> Result<Vec<Category>, AppError> {
    Ok(
        sqlx::query_as::<_, Category>("SELECT * FROM public.categories ORDER BY id")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn create(pool: &PgPool, name: &str) -> Result<Category, AppError> {
    Ok(sqlx::query_as::<_, Category>(
        "INSERT INTO public.categories (name) VALUES ($1) RETURNING *",
    )
    .bind(name)
    .fetch_one(pool)
    .await?)
}
