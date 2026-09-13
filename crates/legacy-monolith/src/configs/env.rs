use wms_core::config::{env_parse_or, env_var, env_var_or};

#[derive(Clone)]
pub struct AppConfig {
    pub server_url: String,
    pub server_port: u16,
    pub jwt_secret: String,

    pub jwt_issuer: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            server_url: env_var_or("SERVER_URL", "0.0.0.0"),
            server_port: env_parse_or("SERVER_PORT", 8080),
            jwt_secret: env_var("JWT_SECRET"),

            jwt_issuer: env_var("JWT_ISSUER"),
        }
    }
}
