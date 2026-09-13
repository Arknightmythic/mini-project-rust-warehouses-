use anyhow::Result;
use sqlx::PgPool;
use wms_core::db::DbConfig;

pub async fn connect() -> Result<PgPool> {
    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    // Migrations run per service, never in the shared crate: this binary embeds
    // only its own schema, resolved from this crate's manifest directory.
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
