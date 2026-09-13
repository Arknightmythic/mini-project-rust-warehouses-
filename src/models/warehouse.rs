use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Warehouse {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub phone: Option<String>,
    pub photo: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    // kept so `SELECT *` maps cleanly onto this struct via FromRow
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub delete_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWarehouseRequest {
    pub name: String,
    pub address: String,
    pub phone: Option<String>,
    pub photo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWarehouseRequest {
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub photo: Option<String>,
}
