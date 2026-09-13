use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};

use crate::models::user::{LoginRequest, LoginResponse, RegisterRequest, User};
use crate::repositories::user_repository;
use crate::state::AppState;
use crate::utils::error::{AppError, AppResult};
use crate::utils::jwt::generate_token;
use crate::utils::password::{hash_password, verify_password};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<User>> {
    if payload.name.trim().is_empty() || payload.email.trim().is_empty() {
        return Err(AppError::Validation(
            "name and email are required".to_string(),
        ));
    }
    if payload.password.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let password_hash = hash_password(&payload.password)?;

    let user = user_repository::create(
        &state.pool,
        &payload.name,
        &payload.email,
        &password_hash,
        payload.phone.as_deref(),
    )
    .await?;

    Ok(Json(user))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let user = user_repository::find_by_email(&state.pool, &payload.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let is_valid = verify_password(&payload.password, &user.password)?;
    if !is_valid {
        return Err(AppError::Unauthorized);
    }

    let roles = user_repository::list_role_names(&state.pool, user.id).await?;
    let token = generate_token(&state.config, user.id, &user.email, roles)?;

    Ok(Json(LoginResponse { token, user }))
}
