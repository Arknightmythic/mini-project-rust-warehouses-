use std::ops::Deref;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use wms_core::{AppError, Identity};

use crate::state::AppState;

// The gateway is the only place a raw JWT is inspected. Everything downstream
// receives identity that has already been verified here.
pub struct AuthUser {
    pub identity: Identity,
    // Kept so it can be forwarded downstream. Services then verify the signature
    // themselves instead of taking the gateway at its word.
    pub token: String,
}

impl Deref for AuthUser {
    type Target = Identity;

    fn deref(&self) -> &Self::Target {
        &self.identity
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Unauthorized)?;

        let claims = wms_core::jwt::verify_token(
            &state.config.jwt_secret,
            &state.config.jwt_issuer,
            bearer.token(),
        )?;

        Ok(AuthUser {
            identity: claims.into(),
            token: bearer.token().to_string(),
        })
    }
}
