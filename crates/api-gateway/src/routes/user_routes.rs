use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::user::v1 as user_v1;

use crate::dto::{RoleJson, UserJson};
use crate::middlewares::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route("/{id}/roles", get(list_user_roles).post(assign_role))
        .route("/{id}/roles/{role_id}", delete(remove_role))
}

#[derive(Deserialize)]
struct UpdateUserBody {
    name: Option<String>,
    phone: Option<String>,
    photo: Option<String>,
}

#[derive(Deserialize)]
struct AssignRoleBody {
    role_id: i64,
}

async fn list_users(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<UserJson>>> {
    let mut client = state.users.clone();

    let users = client
        .list_users(Request::new(user_v1::ListUsersRequest {}))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .users;

    Ok(Json(users.into_iter().map(UserJson::from).collect()))
}

async fn get_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<UserJson>> {
    let mut client = state.users.clone();

    let user = client
        .get_user(Request::new(user_v1::GetUserRequest { id }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(user.into()))
}

async fn update_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserBody>,
) -> AppResult<Json<UserJson>> {
    if auth.user_id != id {
        auth.require_role("admin")?;
    }

    let mut client = state.users.clone();

    let user = client
        .update_user(Request::new(user_v1::UpdateUserRequest {
            id,
            name: body.name,
            phone: body.phone,
            photo: body.photo,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(user.into()))
}

async fn delete_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let mut client = state.users.clone();
    client
        .delete_user(Request::new(user_v1::DeleteUserRequest { id }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "message": "user deleted" })))
}

async fn list_user_roles(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<RoleJson>>> {
    let mut client = state.users.clone();

    let roles = client
        .list_user_roles(Request::new(user_v1::ListUserRolesRequest { user_id: id }))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .roles;

    Ok(Json(roles.into_iter().map(RoleJson::from).collect()))
}

async fn assign_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<AssignRoleBody>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let mut client = state.users.clone();
    client
        .assign_role(Request::new(user_v1::AssignRoleRequest {
            user_id: id,
            role_id: body.role_id,
        }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "message": "role assigned" })))
}

async fn remove_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, role_id)): Path<(i64, i64)>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let mut client = state.users.clone();
    client
        .remove_role(Request::new(user_v1::RemoveRoleRequest {
            user_id: id,
            role_id,
        }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "message": "role removed" })))
}
