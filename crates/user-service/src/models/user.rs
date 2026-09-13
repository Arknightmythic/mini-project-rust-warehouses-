use chrono::{DateTime, Utc};

// No serde here: this struct never leaves the service. What callers see is
// wms.user.v1.User, which has no password field at all — the contract makes
// leaking it impossible rather than merely discouraged.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password: String,
    pub photo: Option<String>,
    pub phone: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
