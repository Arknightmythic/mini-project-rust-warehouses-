use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use crate::state::AppState;
use crate::utils::error::AppError;
use crate::utils::jwt::verify_token;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    // carried for handlers that need the caller's identity, e.g. audit logging
    #[allow(dead_code)]
    pub email: String,
    pub roles: Vec<String>,
}

impl AuthUser {
    pub fn require_role(&self, role: &str) -> Result<(), AppError> {
        if self.roles.iter().any(|r| r == role) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn require_any_role(&self, roles: &[&str]) -> Result<(), AppError> {
        if roles.iter().any(|role| self.roles.iter().any(|r| r == role)) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
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

        let claims = verify_token(&state.config, bearer.token())?;

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
            roles: claims.roles,
        })
    }
}
