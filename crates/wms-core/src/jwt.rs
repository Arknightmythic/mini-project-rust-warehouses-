use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::identity::Identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub email: String,
    pub roles: Vec<String>,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
}

impl From<Claims> for Identity {
    fn from(claims: Claims) -> Self {
        Identity {
            user_id: claims.sub,
            email: claims.email,
            roles: claims.roles,
        }
    }
}

// Only verification lives here. Signing stays with the single service that issues
// tokens, so "one signer, many verifiers" is enforced by where the code sits.
pub fn verify_token(secret: &str, issuer: &str, token: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.set_issuer(&[issuer]);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}
