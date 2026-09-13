use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StockBalance {
    pub warehouse_id: i64,
    pub product_id: i64,
    pub qty_on_hand: i64,
    pub qty_reserved: i64,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StockMovement {
    pub id: i64,
    pub warehouse_id: i64,
    pub product_id: i64,
    pub movement_type: String,
    pub quantity: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InboundReceipt {
    pub id: i64,
    pub warehouse_id: i64,
    pub reference_no: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InboundReceiptItem {
    pub product_id: i64,
    pub quantity: i64,
    pub unit_cost: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OutboundShipment {
    pub id: i64,
    pub warehouse_id: i64,
    pub reference_no: Option<String>,
    // RESERVED / SHIPPED / CANCELLED. This column is the saga state machine.
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ShipmentLine {
    pub product_id: i64,
    pub quantity: i64,
}
