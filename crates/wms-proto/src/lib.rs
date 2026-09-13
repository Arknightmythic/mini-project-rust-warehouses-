pub mod user {
    pub mod v1 {
        include!("generated/wms.user.v1.rs");
    }
}

pub mod warehouse {
    pub mod v1 {
        include!("generated/wms.warehouse.v1.rs");
    }
}

use chrono::{DateTime, Utc};
use prost_types::Timestamp;

// chrono and prost-types have no From impls for each other, so these two live
// here once instead of being rewritten in every service.
pub fn to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

pub fn from_timestamp(ts: &Timestamp) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
}

pub fn opt_timestamp(dt: Option<DateTime<Utc>>) -> Option<Timestamp> {
    dt.map(to_timestamp)
}
