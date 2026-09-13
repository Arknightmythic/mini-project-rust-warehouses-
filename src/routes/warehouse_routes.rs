use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::middlewares::AuthUser;
use crate::models::warehouse::{CreateWarehouseRequest, UpdateWarehouseRequest, Warehouse};
use crate::repositories::warehouse_repository;
use crate::state::AppState;
use crate::utils::error::{AppError, AppResult};

const MANAGER_ROLES: [&str; 2] = ["admin", "warehouse_manager"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_warehouses).post(create_warehouse))
        .route(
            "/{id}",
            get(get_warehouse)
                .put(update_warehouse)
                .delete(delete_warehouse),
        )
}

async fn list_warehouses(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<Warehouse>>> {
    let warehouses = warehouse_repository::list(&state.pool).await?;
    Ok(Json(warehouses))
}

async fn get_warehouse(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Warehouse>> {
    let warehouse = warehouse_repository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("warehouse not found".to_string()))?;
    Ok(Json(warehouse))
}

async fn create_warehouse(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateWarehouseRequest>,
) -> AppResult<Json<Warehouse>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    if payload.name.trim().is_empty() || payload.address.trim().is_empty() {
        return Err(AppError::Validation(
            "name and address are required".to_string(),
        ));
    }

    let warehouse = warehouse_repository::create(
        &state.pool,
        &payload.name,
        &payload.address,
        payload.phone.as_deref(),
        payload.photo.as_deref(),
    )
    .await?;

    Ok(Json(warehouse))
}

async fn update_warehouse(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateWarehouseRequest>,
) -> AppResult<Json<Warehouse>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    let warehouse = warehouse_repository::update(
        &state.pool,
        id,
        payload.name.as_deref(),
        payload.address.as_deref(),
        payload.phone.as_deref(),
        payload.photo.as_deref(),
    )
    .await?
    .ok_or_else(|| AppError::NotFound("warehouse not found".to_string()))?;

    Ok(Json(warehouse))
}

async fn delete_warehouse(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let affected = warehouse_repository::soft_delete(&state.pool, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound("warehouse not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "warehouse deleted" })))
}
