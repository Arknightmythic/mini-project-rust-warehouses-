use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::config::{env_parse_or, env_var};

pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl DbConfig {
    pub fn from_env() -> Self {
        Self {
            url: env_var("DATABASE_URL"),
            max_connections: env_parse_or("DB_MAX_CONNECTIONS", 10),
            acquire_timeout_secs: env_parse_or("DB_CONNECTION_TIMEOUT_SECS", 10),
            idle_timeout_secs: env_parse_or("DB_IDLE_TIMEOUT_SECS", 300),
        }
    }
}

// Deliberately does NOT run migrations: each service owns its own schema, and
// putting migrate!() here would hand every service every other service's tables.
pub async fn connect(config: &DbConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .connect(&config.url)
        .await
}
