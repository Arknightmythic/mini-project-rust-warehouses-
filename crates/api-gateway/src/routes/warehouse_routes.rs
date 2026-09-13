use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::warehouse::v1 as warehouse_v1;

use crate::dto::WarehouseJson;
use crate::middlewares::AuthUser;
use crate::state::AppState;

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

#[derive(Deserialize)]
struct CreateWarehouseBody {
    name: String,
    address: String,
    phone: Option<String>,
    photo: Option<String>,
}

#[derive(Deserialize)]
struct UpdateWarehouseBody {
    name: Option<String>,
    address: Option<String>,
    phone: Option<String>,
    photo: Option<String>,
}

async fn list_warehouses(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<WarehouseJson>>> {
    let mut client = state.warehouses.clone();

    let warehouses = client
        .list_warehouses(Request::new(warehouse_v1::ListWarehousesRequest {}))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .warehouses;

    Ok(Json(
        warehouses.into_iter().map(WarehouseJson::from).collect(),
    ))
}

async fn get_warehouse(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<WarehouseJson>> {
    let mut client = state.warehouses.clone();

    let warehouse = client
        .get_warehouse(Request::new(warehouse_v1::GetWarehouseRequest { id }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(warehouse.into()))
}

async fn create_warehouse(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateWarehouseBody>,
) -> AppResult<Json<WarehouseJson>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    let mut client = state.warehouses.clone();

    let warehouse = client
        .create_warehouse(Request::new(warehouse_v1::CreateWarehouseRequest {
            name: body.name,
            address: body.address,
            phone: body.phone,
            photo: body.photo,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(warehouse.into()))
}

async fn update_warehouse(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateWarehouseBody>,
) -> AppResult<Json<WarehouseJson>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    let mut client = state.warehouses.clone();

    let warehouse = client
        .update_warehouse(Request::new(warehouse_v1::UpdateWarehouseRequest {
            id,
            name: body.name,
            address: body.address,
            phone: body.phone,
            photo: body.photo,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(warehouse.into()))
}

async fn delete_warehouse(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let mut client = state.warehouses.clone();
    client
        .delete_warehouse(Request::new(warehouse_v1::DeleteWarehouseRequest { id }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "message": "warehouse deleted" })))
}
