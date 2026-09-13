use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::inventory::v1 as inventory_v1;

use crate::dto::{ReceiptJson, StockBalanceJson, StockMovementJson};
use crate::middlewares::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/receipts", post(receive_stock))
        .route("/balances", get(list_balances))
        .route("/balances/{warehouse_id}/{product_id}", get(get_balance))
        .route("/movements", get(list_movements))
}

#[derive(Deserialize)]
struct ReceiptItemBody {
    product_id: i64,
    quantity: i64,
    #[serde(default)]
    unit_cost: i64,
}

#[derive(Deserialize)]
struct ReceiveStockBody {
    warehouse_id: i64,
    reference_no: Option<String>,
    items: Vec<ReceiptItemBody>,
    idempotency_key: String,
}

#[derive(Deserialize)]
struct BalanceQuery {
    warehouse_id: Option<i64>,
}

#[derive(Deserialize)]
struct MovementQuery {
    warehouse_id: Option<i64>,
    product_id: Option<i64>,
}

async fn receive_stock(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ReceiveStockBody>,
) -> AppResult<Json<ReceiptJson>> {
    let mut client = state.inventory.clone();

    let mut request = Request::new(inventory_v1::ReceiveStockRequest {
        warehouse_id: body.warehouse_id,
        reference_no: body.reference_no,
        items: body
            .items
            .into_iter()
            .map(|item| inventory_v1::ReceiptItem {
                product_id: item.product_id,
                quantity: item.quantity,
                unit_cost: item.unit_cost,
            })
            .collect(),
        idempotency_key: body.idempotency_key,
    });

    // Identity is attached explicitly, not by an interceptor. Hiding a trust
    // boundary behind magic is the wrong kind of convenience: the call site
    // should show that this gateway is vouching for the caller.
    wms_core::grpc::identity_to_metadata(&auth, request.metadata_mut());

    let response = client
        .receive_stock(request)
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(response.into()))
}

async fn list_balances(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<BalanceQuery>,
) -> AppResult<Json<Vec<StockBalanceJson>>> {
    let mut client = state.inventory.clone();

    let balances = client
        .list_stock_balances(Request::new(inventory_v1::ListStockBalancesRequest {
            warehouse_id: query.warehouse_id,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .balances;

    Ok(Json(
        balances.into_iter().map(StockBalanceJson::from).collect(),
    ))
}

async fn get_balance(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((warehouse_id, product_id)): Path<(i64, i64)>,
) -> AppResult<Json<StockBalanceJson>> {
    let mut client = state.inventory.clone();

    let balance = client
        .get_stock_balance(Request::new(inventory_v1::GetStockBalanceRequest {
            warehouse_id,
            product_id,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(balance.into()))
}

async fn list_movements(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<MovementQuery>,
) -> AppResult<Json<Vec<StockMovementJson>>> {
    let mut client = state.inventory.clone();

    let movements = client
        .list_stock_movements(Request::new(inventory_v1::ListStockMovementsRequest {
            warehouse_id: query.warehouse_id,
            product_id: query.product_id,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .movements;

    Ok(Json(
        movements.into_iter().map(StockMovementJson::from).collect(),
    ))
}
