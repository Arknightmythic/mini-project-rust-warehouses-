use jsonwebtoken::{EncodingKey, Header, encode};
use wms_core::AppError;
use wms_core::jwt::Claims;

use crate::config::ServiceConfig;

// This service is the only token issuer in the system. wms-core deliberately
// ships verification only, so "one signer, many verifiers" is enforced by layout.
pub fn generate_token(
    config: &ServiceConfig,
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
