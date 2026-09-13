use wms_core::config::env_var_or;

#[derive(Clone)]
pub struct ServiceConfig {
    pub grpc_addr: String,
    pub warehouse_service_url: String,
    pub product_service_url: String,
    pub amqp_url: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub sweeper_interval_secs: u64,
    pub reservation_timeout_secs: i64,
}

impl ServiceConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_addr: env_var_or("GRPC_ADDR", "0.0.0.0:50054"),
            warehouse_service_url: env_var_or("WAREHOUSE_SERVICE_URL", "http://127.0.0.1:50052"),
            product_service_url: env_var_or("PRODUCT_SERVICE_URL", "http://127.0.0.1:50053"),
            amqp_url: env_var_or("AMQP_URL", ""),
            jwt_secret: wms_core::config::env_var("JWT_SECRET"),
            jwt_issuer: wms_core::config::env_var("JWT_ISSUER"),
            sweeper_interval_secs: wms_core::config::env_parse_or("SWEEPER_INTERVAL_SECS", 30),
            reservation_timeout_secs: wms_core::config::env_parse_or("RESERVATION_TIMEOUT_SECS", 300),
        }
    }
}
