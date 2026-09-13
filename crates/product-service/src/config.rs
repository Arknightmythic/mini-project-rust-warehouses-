use wms_core::config::env_var_or;

#[derive(Clone)]
pub struct ServiceConfig {
    pub grpc_addr: String,
}

impl ServiceConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_addr: env_var_or("GRPC_ADDR", "0.0.0.0:50053"),
        }
    }
}
