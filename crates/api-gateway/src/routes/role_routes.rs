use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::user::v1 as user_v1;

use crate::dto::RoleJson;
use crate::middlewares::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/{id}", get(get_role).put(update_role).delete(delete_role))
}

#[derive(Deserialize)]
struct RoleBody {
    name: String,
}

async fn list_roles(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<RoleJson>>> {
    let mut client = state.users.clone();

    let roles = client
        .list_roles(Request::new(user_v1::ListRolesRequest {}))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .roles;

    Ok(Json(roles.into_iter().map(RoleJson::from).collect()))
}

async fn get_role(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<RoleJson>> {
    let mut client = state.users.clone();

    let role = client
        .get_role(Request::new(user_v1::GetRoleRequest { id }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(role.into()))
}

async fn create_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<RoleBody>,
) -> AppResult<Json<RoleJson>> {
    auth.require_role("admin")?;

    let mut client = state.users.clone();

    let role = client
        .create_role(Request::new(user_v1::CreateRoleRequest { name: body.name }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(role.into()))
}

async fn update_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<RoleBody>,
) -> AppResult<Json<RoleJson>> {
    auth.require_role("admin")?;

    let mut client = state.users.clone();

    let role = client
        .update_role(Request::new(user_v1::UpdateRoleRequest {
            id,
            name: body.name,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(role.into()))
}

async fn delete_role(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let mut client = state.users.clone();
    client
        .delete_role(Request::new(user_v1::DeleteRoleRequest { id }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "message": "role deleted" })))
}
