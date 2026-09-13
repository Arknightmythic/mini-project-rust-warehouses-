use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::inventory::v1 as inventory_v1;

use wms_proto::product::v1 as product_v1;

use crate::dto::{
    ReceiptJson, ShipmentJson, StockBalanceJson, StockMovementJson, StockReportRowJson,
};
use crate::middlewares::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/receipts", post(receive_stock))
        .route("/balances", get(list_balances))
        .route("/balances/{warehouse_id}/{product_id}", get(get_balance))
        .route("/movements", get(list_movements))
        .route("/report", get(stock_report))
        .route("/shipments", post(ship_stock))
        .route("/shipments/{id}", get(get_shipment))
        .route("/shipments/{id}/confirm", post(confirm_shipment))
        .route("/shipments/{id}/cancel", post(cancel_shipment))
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
    wms_core::grpc::token_to_metadata(&auth.token, request.metadata_mut());

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

// Two calls, deliberately: one to the owner of quantities, one to the owner of
// names, then a join in memory. The batch RPC keeps it at two round trips no
// matter how many rows come back.
async fn stock_report(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<BalanceQuery>,
) -> AppResult<Json<Vec<StockReportRowJson>>> {
    let mut inventory = state.inventory.clone();

    let balances = inventory
        .list_stock_balances(Request::new(inventory_v1::ListStockBalancesRequest {
            warehouse_id: query.warehouse_id,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .balances;

    let mut product_ids: Vec<i64> = balances.iter().map(|b| b.product_id).collect();
    product_ids.sort_unstable();
    product_ids.dedup();

    let mut products = state.products.clone();
    let catalog = products
        .get_products_by_ids(Request::new(product_v1::GetProductsByIdsRequest {
            ids: product_ids,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .products;

    let rows = balances
        .into_iter()
        .map(|balance| {
            let product = catalog.iter().find(|p| p.id == balance.product_id);

            StockReportRowJson {
                warehouse_id: balance.warehouse_id,
                product_id: balance.product_id,
                // None when the catalog has no such product. Without foreign keys
                // across services this is a real state, so the API admits it
                // instead of inventing a name.
                sku: product.map(|p| p.sku.clone()),
                product_name: product.map(|p| p.name.clone()),
                unit: product.map(|p| p.unit.clone()),
                qty_on_hand: balance.qty_on_hand,
                qty_reserved: balance.qty_reserved,
                qty_available: balance.qty_available,
            }
        })
        .collect();

    Ok(Json(rows))
}

#[derive(Deserialize)]
struct ShipStockBody {
    warehouse_id: i64,
    reference_no: Option<String>,
    items: Vec<ShipmentItemBody>,
    idempotency_key: String,
}

#[derive(Deserialize)]
struct ShipmentItemBody {
    product_id: i64,
    quantity: i64,
}

#[derive(Deserialize)]
struct CancelBody {
    reason: Option<String>,
}

// Step 1: reserve. Returns RESERVED, not SHIPPED - nothing has left the warehouse.
async fn ship_stock(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ShipStockBody>,
) -> AppResult<Json<ShipmentJson>> {
    let mut client = state.inventory.clone();

    let mut request = Request::new(inventory_v1::ShipStockRequest {
        warehouse_id: body.warehouse_id,
        reference_no: body.reference_no,
        items: body
            .items
            .into_iter()
            .map(|item| inventory_v1::ShipmentItem {
                product_id: item.product_id,
                quantity: item.quantity,
            })
            .collect(),
        idempotency_key: body.idempotency_key,
    });
    wms_core::grpc::identity_to_metadata(&auth, request.metadata_mut());
    wms_core::grpc::token_to_metadata(&auth.token, request.metadata_mut());

    let shipment = client
        .ship_stock(request)
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(shipment.into()))
}

// Step 2: commit.
async fn confirm_shipment(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ShipmentJson>> {
    let mut client = state.inventory.clone();

    let mut request = Request::new(inventory_v1::ConfirmShipmentRequest { shipment_id: id });
    wms_core::grpc::identity_to_metadata(&auth, request.metadata_mut());
    wms_core::grpc::token_to_metadata(&auth.token, request.metadata_mut());

    let shipment = client
        .confirm_shipment(request)
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(shipment.into()))
}

// Step 3: compensate.
async fn cancel_shipment(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<CancelBody>,
) -> AppResult<Json<ShipmentJson>> {
    let mut client = state.inventory.clone();

    let mut request = Request::new(inventory_v1::CancelShipmentRequest {
        shipment_id: id,
        reason: body.reason,
    });
    wms_core::grpc::identity_to_metadata(&auth, request.metadata_mut());
    wms_core::grpc::token_to_metadata(&auth.token, request.metadata_mut());

    let shipment = client
        .cancel_shipment(request)
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(shipment.into()))
}

async fn get_shipment(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ShipmentJson>> {
    let mut client = state.inventory.clone();

    let shipment = client
        .get_shipment(Request::new(inventory_v1::GetShipmentRequest { shipment_id: id }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(shipment.into()))
}
