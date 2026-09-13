use wms_core::config::{env_parse_or, env_var, env_var_or};

#[derive(Clone)]
pub struct ServiceConfig {
    pub grpc_addr: String,
    pub jwt_secret: String,
    pub jwt_expiration_minutes: i64,
    pub jwt_issuer: String,
}

impl ServiceConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_addr: env_var_or("GRPC_ADDR", "0.0.0.0:50051"),
            jwt_secret: env_var("JWT_SECRET"),
            jwt_expiration_minutes: env_parse_or("JWT_EXPIRATION_MINUTES", 60),
            jwt_issuer: env_var("JWT_ISSUER"),
        }
    }
}
