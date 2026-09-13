use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use super::env::AppConfig;

pub async fn connect(config: &AppConfig) -> Result<PgPool, sqlx::Error> {
    let idle_timeout_secs: u64 = std::env::var("DB_IDLE_TIMEOUT_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .expect("DB_IDLE_TIMEOUT_SECS must be a valid u64");
    let connect_timeout_secs: u64 = std::env::var("DB_CONNECTION_TIMEOUT_SECS")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("DB_CONNECTION_TIMEOUT_SECS must be a valid u64");

    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(connect_timeout_secs))
        .idle_timeout(Duration::from_secs(idle_timeout_secs))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
