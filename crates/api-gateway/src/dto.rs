use chrono::{DateTime, Utc};
use serde::Serialize;
use wms_proto::user::v1 as user_v1;
use wms_proto::warehouse::v1 as warehouse_v1;

// protobuf timestamps are seconds+nanos; chrono serde renders RFC3339, which is
// what v1 emitted. Converting here keeps the public JSON byte-identical while the
// wire format between services changed completely.
#[derive(Serialize)]
pub struct UserJson {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub photo: Option<String>,
    pub phone: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<user_v1::User> for UserJson {
    fn from(user: user_v1::User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            photo: user.photo,
            phone: user.phone,
            created_at: user.created_at.as_ref().and_then(wms_proto::from_timestamp),
            updated_at: user.updated_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}

#[derive(Serialize)]
pub struct RoleJson {
    pub id: i64,
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<user_v1::Role> for RoleJson {
    fn from(role: user_v1::Role) -> Self {
        Self {
            id: role.id,
            name: role.name,
            created_at: role.created_at.as_ref().and_then(wms_proto::from_timestamp),
            updated_at: role.updated_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}

#[derive(Serialize)]
pub struct LoginJson {
    pub token: String,
    pub user: UserJson,
}

#[derive(Serialize)]
pub struct WarehouseJson {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub phone: Option<String>,
    pub photo: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<warehouse_v1::Warehouse> for WarehouseJson {
    fn from(warehouse: warehouse_v1::Warehouse) -> Self {
        Self {
            id: warehouse.id,
            name: warehouse.name,
            address: warehouse.address,
            phone: warehouse.phone,
            photo: warehouse.photo,
            created_at: warehouse.created_at.as_ref().and_then(wms_proto::from_timestamp),
            updated_at: warehouse.updated_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}
