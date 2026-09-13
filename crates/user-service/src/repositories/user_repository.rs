use sqlx::PgPool;
use wms_core::AppError;

use crate::models::user::User;

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM public.users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<User>, AppError> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM public.users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn list(pool: &PgPool) -> Result<Vec<User>, AppError> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM public.users ORDER BY id")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
    phone: Option<&str>,
) -> Result<User, AppError> {
    Ok(sqlx::query_as::<_, User>(
        "INSERT INTO public.users (name, email, password, phone)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .bind(phone)
    .fetch_one(pool)
    .await?)
}

pub async fn update(
    pool: &PgPool,
    id: i64,
    name: Option<&str>,
    phone: Option<&str>,
    photo: Option<&str>,
) -> Result<Option<User>, AppError> {
    Ok(sqlx::query_as::<_, User>(
        "UPDATE public.users SET
            name = COALESCE($2, name),
            phone = COALESCE($3, phone),
            photo = COALESCE($4, photo),
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(phone)
    .bind(photo)
    .fetch_optional(pool)
    .await?)
}

pub async fn delete(pool: &PgPool, id: i64) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM public.users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn list_role_names(pool: &PgPool, user_id: i64) -> Result<Vec<String>, AppError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT r.name FROM public.roles r
         JOIN public.user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}
