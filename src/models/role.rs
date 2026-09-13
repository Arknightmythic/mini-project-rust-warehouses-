use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: String,
}
