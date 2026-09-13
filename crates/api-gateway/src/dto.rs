use chrono::{DateTime, Utc};
use serde::Serialize;
use wms_proto::user::v1 as user_v1;
use wms_proto::inventory::v1 as inventory_v1;
use wms_proto::product::v1 as product_v1;
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

// Stock balances joined with product names. The join happens HERE, in the gateway,
// because neither service may reach into the other's database and neither should
// grow a read-path dependency on the other just to render a list.
//
// The rule this endpoint demonstrates: reads compose at the gateway, writes
// validate at the owner.
#[derive(Serialize)]
pub struct StockReportRowJson {
    pub warehouse_id: i64,
    pub product_id: i64,
    pub sku: Option<String>,
    pub product_name: Option<String>,
    pub unit: Option<String>,
    pub qty_on_hand: i64,
    pub qty_reserved: i64,
    pub qty_available: i64,
}

#[derive(Serialize)]
pub struct ProductJson {
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
    // Read model owned by inventory-service, copied here by events. Shown with an
    // explicit sync timestamp so a stale value is visible rather than misleading.
    pub total_stock_cached: i64,
    pub stock_synced_at: Option<DateTime<Utc>>,
}

impl From<product_v1::Product> for ProductJson {
    fn from(product: product_v1::Product) -> Self {
        Self {
            id: product.id,
            sku: product.sku,
            name: product.name,
            description: product.description,
            unit: product.unit,
            barcode: product.barcode,
            category_id: product.category_id,
            is_active: product.is_active,
            created_at: product.created_at.as_ref().and_then(wms_proto::from_timestamp),
            updated_at: product.updated_at.as_ref().and_then(wms_proto::from_timestamp),
            total_stock_cached: product.total_stock_cached,
            stock_synced_at: product.stock_synced_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}

#[derive(Serialize)]
pub struct CategoryJson {
    pub id: i64,
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<product_v1::Category> for CategoryJson {
    fn from(category: product_v1::Category) -> Self {
        Self {
            id: category.id,
            name: category.name,
            created_at: category.created_at.as_ref().and_then(wms_proto::from_timestamp),
            updated_at: category.updated_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}

#[derive(Serialize)]
pub struct ReceiptItemJson {
    pub product_id: i64,
    pub quantity: i64,
    pub unit_cost: i64,
}

#[derive(Serialize)]
pub struct ReceiptJson {
    pub receipt_id: i64,
    pub warehouse_id: i64,
    pub reference_no: Option<String>,
    pub items: Vec<ReceiptItemJson>,
    pub created_at: Option<DateTime<Utc>>,
    pub idempotent_replay: bool,
}

impl From<inventory_v1::ReceiveStockResponse> for ReceiptJson {
    fn from(response: inventory_v1::ReceiveStockResponse) -> Self {
        Self {
            receipt_id: response.receipt_id,
            warehouse_id: response.warehouse_id,
            reference_no: response.reference_no,
            items: response
                .items
                .into_iter()
                .map(|item| ReceiptItemJson {
                    product_id: item.product_id,
                    quantity: item.quantity,
                    unit_cost: item.unit_cost,
                })
                .collect(),
            created_at: response.created_at.as_ref().and_then(wms_proto::from_timestamp),
            idempotent_replay: response.idempotent_replay,
        }
    }
}

#[derive(Serialize)]
pub struct StockBalanceJson {
    pub warehouse_id: i64,
    pub product_id: i64,
    pub qty_on_hand: i64,
    pub qty_reserved: i64,
    pub qty_available: i64,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<inventory_v1::StockBalance> for StockBalanceJson {
    fn from(balance: inventory_v1::StockBalance) -> Self {
        Self {
            warehouse_id: balance.warehouse_id,
            product_id: balance.product_id,
            qty_on_hand: balance.qty_on_hand,
            qty_reserved: balance.qty_reserved,
            qty_available: balance.qty_available,
            updated_at: balance.updated_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}

#[derive(Serialize)]
pub struct StockMovementJson {
    pub id: i64,
    pub warehouse_id: i64,
    pub product_id: i64,
    pub movement_type: String,
    pub quantity: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<inventory_v1::StockMovement> for StockMovementJson {
    fn from(movement: inventory_v1::StockMovement) -> Self {
        Self {
            id: movement.id,
            warehouse_id: movement.warehouse_id,
            product_id: movement.product_id,
            movement_type: movement.movement_type,
            quantity: movement.quantity,
            reference_type: movement.reference_type,
            reference_id: movement.reference_id,
            created_at: movement.created_at.as_ref().and_then(wms_proto::from_timestamp),
        }
    }
}
