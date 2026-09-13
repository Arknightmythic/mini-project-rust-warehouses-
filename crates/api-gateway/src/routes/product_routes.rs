use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tonic::Request;
use wms_core::{AppError, AppResult};
use wms_proto::product::v1 as product_v1;

use crate::dto::{CategoryJson, ProductJson};
use crate::middlewares::AuthUser;
use crate::state::AppState;

const MANAGER_ROLES: [&str; 2] = ["admin", "warehouse_manager"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_products).post(create_product))
        .route(
            "/{id}",
            get(get_product).put(update_product).delete(deactivate_product),
        )
}

pub fn category_router() -> Router<AppState> {
    Router::new().route("/", get(list_categories).post(create_category))
}

#[derive(Deserialize)]
struct ListProductsQuery {
    #[serde(default)]
    include_inactive: bool,
}

#[derive(Deserialize)]
struct CreateProductBody {
    sku: String,
    name: String,
    description: Option<String>,
    unit: Option<String>,
    barcode: Option<String>,
    category_id: Option<i64>,
}

#[derive(Deserialize)]
struct UpdateProductBody {
    name: Option<String>,
    description: Option<String>,
    unit: Option<String>,
    barcode: Option<String>,
    category_id: Option<i64>,
}

#[derive(Deserialize)]
struct CategoryBody {
    name: String,
}

async fn list_products(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListProductsQuery>,
) -> AppResult<Json<Vec<ProductJson>>> {
    let mut client = state.products.clone();

    let products = client
        .list_products(Request::new(product_v1::ListProductsRequest {
            include_inactive: query.include_inactive,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .products;

    Ok(Json(products.into_iter().map(ProductJson::from).collect()))
}

async fn get_product(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ProductJson>> {
    let mut client = state.products.clone();

    let product = client
        .get_product(Request::new(product_v1::GetProductRequest { id }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(product.into()))
}

async fn create_product(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateProductBody>,
) -> AppResult<Json<ProductJson>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    let mut client = state.products.clone();

    let product = client
        .create_product(Request::new(product_v1::CreateProductRequest {
            sku: body.sku,
            name: body.name,
            description: body.description,
            unit: body.unit,
            barcode: body.barcode,
            category_id: body.category_id,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(product.into()))
}

async fn update_product(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateProductBody>,
) -> AppResult<Json<ProductJson>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    let mut client = state.products.clone();

    let product = client
        .update_product(Request::new(product_v1::UpdateProductRequest {
            id,
            name: body.name,
            description: body.description,
            unit: body.unit,
            barcode: body.barcode,
            category_id: body.category_id,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(product.into()))
}

async fn deactivate_product(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_role("admin")?;

    let mut client = state.products.clone();
    client
        .deactivate_product(Request::new(product_v1::DeactivateProductRequest { id }))
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "message": "product deactivated" })))
}

async fn list_categories(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CategoryJson>>> {
    let mut client = state.products.clone();

    let categories = client
        .list_categories(Request::new(product_v1::ListCategoriesRequest {}))
        .await
        .map_err(AppError::from)?
        .into_inner()
        .categories;

    Ok(Json(categories.into_iter().map(CategoryJson::from).collect()))
}

async fn create_category(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CategoryBody>,
) -> AppResult<Json<CategoryJson>> {
    auth.require_any_role(&MANAGER_ROLES)?;

    let mut client = state.products.clone();

    let category = client
        .create_category(Request::new(product_v1::CreateCategoryRequest {
            name: body.name,
        }))
        .await
        .map_err(AppError::from)?
        .into_inner();

    Ok(Json(category.into()))
}
