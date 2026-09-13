use sqlx::PgPool;

use crate::models::role::Role;

pub async fn assign(pool: &PgPool, user_id: i64, role_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO public.user_roles (user_id, role_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id, role_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(role_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove(pool: &PgPool, user_id: i64, role_id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM public.user_roles WHERE user_id = $1 AND role_id = $2")
        .bind(user_id)
        .bind(role_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn list_roles_for_user(pool: &PgPool, user_id: i64) -> Result<Vec<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>(
        "SELECT r.* FROM public.roles r
         JOIN public.user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = $1
         ORDER BY r.id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}
