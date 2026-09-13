use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub phone: Option<String>,
    pub photo: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    // Column name kept as-is from v1 (delete_at, not deleted_at). Renaming it here
    // would mix a schema change into a phase that should be pure mechanics.
    #[allow(dead_code)]
    pub delete_at: Option<DateTime<Utc>>,
}
