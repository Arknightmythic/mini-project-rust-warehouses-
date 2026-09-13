use wms_core::config::env_var_or;

#[derive(Clone)]
pub struct ServiceConfig {
    pub grpc_addr: String,
    pub warehouse_service_url: String,
    pub product_service_url: String,
}

impl ServiceConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_addr: env_var_or("GRPC_ADDR", "0.0.0.0:50054"),
            warehouse_service_url: env_var_or("WAREHOUSE_SERVICE_URL", "http://127.0.0.1:50052"),
            product_service_url: env_var_or("PRODUCT_SERVICE_URL", "http://127.0.0.1:50053"),
        }
    }
}
