#[derive(Clone)]
pub struct AppConfig {
    pub server_url: String,
    pub server_port: u16,
    pub database_url: String,
    pub db_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_expiration_minutes: i64,
    pub jwt_issuer: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            server_url: std::env::var("SERVER_URL").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("SERVER_PORT must be a valid u16"),
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            db_max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .expect("DB_MAX_CONNECTIONS must be a valid u32"),
            jwt_secret: std::env::var("Jwt_SECRET").expect("Jwt_SECRET must be set"),
            jwt_expiration_minutes: std::env::var("JWT_EXPIRATION_MINUTES")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .expect("JWT_EXPIRATION_MINUTES must be a valid i64"),
            jwt_issuer: std::env::var("JWT_ISSUER").expect("JWT_ISSUER must be set"),
        }
    }
}
