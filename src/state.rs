use std::sync::Arc;

use sqlx::PgPool;

use crate::configs::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<AppConfig>,
}
