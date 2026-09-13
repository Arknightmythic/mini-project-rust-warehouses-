use wms_core::config::env_var_or;

#[derive(Clone)]
pub struct ServiceConfig {
    pub amqp_url: String,
    pub user_service_url: String,
}

impl ServiceConfig {
    pub fn from_env() -> Self {
        Self {
            amqp_url: env_var_or("AMQP_URL", "amqp://guest:guest@127.0.0.1:5672/%2f"),
            user_service_url: env_var_or("USER_SERVICE_URL", "http://127.0.0.1:50051"),
        }
    }
}
