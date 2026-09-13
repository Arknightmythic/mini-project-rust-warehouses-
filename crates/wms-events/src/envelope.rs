use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Every message carries the same outer shape. The payload varies; the metadata
// that makes the system debuggable and safe to retry does not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    // UUIDv7 is time-sortable, so ordering event ids also orders them in time.
    // This is the key the consumer deduplicates on.
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    // W3C traceparent. Carried inside the JSON body for now, which is not the
    // standard place for it - real AMQP headers are - but it links the trace in
    // five lines instead of forty, and moving it later is a contained change.
    pub trace_parent: Option<String>,
    pub payload: T,
}

impl<T> Envelope<T> {
    pub fn new(event_type: &str, payload: T) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            event_type: event_type.to_string(),
            occurred_at: Utc::now(),
            trace_parent: None,
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedLine {
    pub product_id: i64,
    pub quantity: i64,
}

// Events describe what HAPPENED, in the language of the service that owns the
// fact. They are not commands: nothing here tells the consumer what to do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockReceived {
    pub receipt_id: i64,
    pub warehouse_id: i64,
    pub reference_no: Option<String>,
    pub received_by: i64,
    pub lines: Vec<ReceivedLine>,
}
