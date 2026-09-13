use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// serde here is for the cache, not for any API: JSON keeps entries readable with
// a plain `redis-cli GET product:1`, which is worth a lot while learning.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: String,
    pub barcode: Option<String>,
    pub category_id: Option<i64>,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub total_stock_cached: i64,
    pub stock_synced_at: Option<DateTime<Utc>>,
}
