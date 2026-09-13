use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::middlewares::AuthUser;
use crate::models::role::{CreateRoleRequest, Role, UpdateRoleRequest};
use crate::repositories::role_repository;
use crate::state::AppState;
use crate::utils::error::{AppError, AppResult};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route(
            "/{id}",
            get(get_role).put(update_role).delete(delete_role),
        )
}

async fn list_roles(_auth: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<Role>>> {
    let roles = role_repository::list(&state.pool).await?;
    Ok(Json(roles))
}

async fn get_role(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Role>> {
    let role = role_repository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("role not found".to_string()))?;
    Ok(Json(role))
}

async fn create_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateRoleRequest>,
) -> AppResult<Json<Role>> {
    auth.require_role("admin")?;

    if payload.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".to_string()));
    }

    let role = role_repository::create(&state.pool, &payload.name).await?;
    Ok(Json(role))
}

async fn update_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateRoleRequest>,
) -> AppResult<Json<Role>> {
    auth.require_role("admin")?;

    let role = role_repository::update(&state.pool, id, &payload.name)
        .await?
        .ok_or_else(|| AppError::NotFound("role not found".to_string()))?;

    Ok(Json(role))
}

async fn delete_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let affected = role_repository::delete(&state.pool, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound("role not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "role deleted" })))
}
