use wms_core::config::env_var_or;

#[derive(Clone)]
pub struct ServiceConfig {
    pub grpc_addr: String,
    pub redis_url: String,
    pub cache_ttl_secs: u64,
    pub amqp_url: String,
}

impl ServiceConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_addr: env_var_or("GRPC_ADDR", "0.0.0.0:50053"),
            redis_url: env_var_or("REDIS_URL", ""),
            cache_ttl_secs: wms_core::config::env_parse_or("CACHE_TTL_SECS", 60),
            amqp_url: env_var_or("AMQP_URL", ""),
        }
    }
}
