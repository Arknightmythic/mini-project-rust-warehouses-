use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::configs::AppConfig;
use crate::utils::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,
    pub email: String,
    pub roles: Vec<String>,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
}

pub fn generate_token(
    config: &AppConfig,
    user_id: i64,
    email: &str,
    roles: Vec<String>,
) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::minutes(config.jwt_expiration_minutes);

    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        roles,
        iss: config.jwt_issuer.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
}

pub fn verify_token(config: &AppConfig, token: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.set_issuer(&[config.jwt_issuer.clone()]);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}
