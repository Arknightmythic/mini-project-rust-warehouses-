use wms_core::config::{env_var, env_var_or};

#[derive(Clone)]
pub struct GatewayConfig {
    pub server_url: String,
    pub server_port: u16,
    pub user_service_url: String,
    pub warehouse_service_url: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
}

impl GatewayConfig {
    pub fn from_env() -> Self {
        Self {
            server_url: env_var_or("SERVER_URL", "0.0.0.0"),
            server_port: wms_core::config::env_parse_or("SERVER_PORT", 8080),
            user_service_url: env_var_or("USER_SERVICE_URL", "http://127.0.0.1:50051"),
            warehouse_service_url: env_var_or("WAREHOUSE_SERVICE_URL", "http://127.0.0.1:50052"),
            jwt_secret: env_var("JWT_SECRET"),
            jwt_issuer: env_var("JWT_ISSUER"),
        }
    }
}
