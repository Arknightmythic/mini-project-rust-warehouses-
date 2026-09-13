use std::ops::Deref;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use wms_core::{AppError, Identity};

use crate::state::AppState;

// Identity belongs to wms-core and FromRequestParts belongs to axum, so the orphan
// rule rules out implementing the trait for Identity here. A local newtype is the
// way through, and Deref keeps handlers reading `auth.user_id` / `auth.require_role`
// exactly as before.
pub struct AuthUser(pub Identity);

impl Deref for AuthUser {
    type Target = Identity;

    fn deref(&self) -> &Self::Target {
        &self.0
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

        Ok(AuthUser(claims.into()))
    }
}
