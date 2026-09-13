use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};

use crate::middlewares::AuthUser;
use crate::models::role::Role;
use crate::models::user::{UpdateUserRequest, User};
use crate::models::user_role::AssignRoleRequest;
use crate::repositories::{user_repository, user_role_repository};
use crate::state::AppState;
use crate::utils::error::{AppError, AppResult};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route("/{id}/roles", get(list_user_roles).post(assign_role))
        .route("/{id}/roles/{role_id}", delete(remove_role))
}

async fn list_users(_auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<User>>> {
    let users = user_repository::list(&state.pool).await?;
    Ok(Json(users))
}

async fn get_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<User>> {
    let user = user_repository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;
    Ok(Json(user))
}

async fn update_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<User>> {
    if auth.user_id != id {
        auth.require_role("admin")?;
    }

    let user = user_repository::update(
        &state.pool,
        id,
        payload.name.as_deref(),
        payload.phone.as_deref(),
        payload.photo.as_deref(),
    )
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    Ok(Json(user))
}

async fn delete_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let affected = user_repository::delete(&state.pool, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound("user not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "user deleted" })))
}

async fn list_user_roles(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<Role>>> {
    let roles = user_role_repository::list_roles_for_user(&state.pool, id).await?;
    Ok(Json(roles))
}

async fn assign_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<AssignRoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    user_role_repository::assign(&state.pool, id, payload.role_id).await?;

    Ok(Json(serde_json::json!({ "message": "role assigned" })))
}

async fn remove_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, role_id)): Path<(i64, i64)>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let affected = user_role_repository::remove(&state.pool, id, role_id).await?;
    if affected == 0 {
        return Err(AppError::NotFound("user role not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "role removed" })))
}
