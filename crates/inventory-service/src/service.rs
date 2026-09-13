use sqlx::PgPool;
use wms_core::grpc::TracedChannel;
use tonic::{Request, Response, Status};
use wms_core::AppError;
use wms_proto::inventory::v1::inventory_service_server::InventoryService;
use wms_proto::inventory::v1::{
    GetStockBalanceRequest, ListStockBalancesRequest, ListStockBalancesResponse,
    ListStockMovementsRequest, ListStockMovementsResponse, ReceiptItem, ReceiveStockRequest,
    ReceiveStockResponse, StockBalance, StockMovement,
};
use wms_proto::product::v1 as product_v1;
use wms_proto::product::v1::product_service_client::ProductServiceClient;
use wms_proto::warehouse::v1 as warehouse_v1;
use wms_proto::warehouse::v1::warehouse_service_client::WarehouseServiceClient;

use crate::models;
use crate::repositories::inventory_repository::{self, NewReceiptItem};

const RECEIVER_ROLES: [&str; 3] = ["admin", "warehouse_manager", "staff"];

// Only Clone + Send + Sync values live here. tonic channels are cheap to clone
// and multiplex over a single HTTP/2 connection.
pub struct InventoryGrpcService {
    pool: PgPool,
    warehouses: WarehouseServiceClient<TracedChannel>,
    products: ProductServiceClient<TracedChannel>,
}

impl InventoryGrpcService {
    pub fn new(
        pool: PgPool,
        warehouses: WarehouseServiceClient<TracedChannel>,
        products: ProductServiceClient<TracedChannel>,
    ) -> Self {
        Self {
            pool,
            warehouses,
            products,
        }
    }

    async fn assert_warehouse_exists(&self, warehouse_id: i64) -> Result<(), AppError> {
        let mut client = self.warehouses.clone();

        let response = client
            .warehouse_exists(Request::new(warehouse_v1::WarehouseExistsRequest {
                id: warehouse_id,
            }))
            .await
            .map_err(AppError::from)?
            .into_inner();

        if response.exists {
            Ok(())
        } else {
            Err(AppError::Validation(format!(
                "warehouse {warehouse_id} not found"
            )))
        }
    }

    // Was one round trip per line item. Jaeger showed the ladder of sequential
    // get_product spans, so it is now a single batch call: N network hops become 1.
    async fn assert_products_exist(&self, product_ids: &[i64]) -> Result<(), AppError> {
        let mut client = self.products.clone();

        let found = client
            .get_products_by_ids(Request::new(product_v1::GetProductsByIdsRequest {
                ids: product_ids.to_vec(),
            }))
            .await
            .map_err(AppError::from)?
            .into_inner()
            .products;

        for product_id in product_ids {
            match found.iter().find(|product| product.id == *product_id) {
                None => {
                    return Err(AppError::Validation(format!(
                        "product {product_id} not found"
                    )));
                }
                Some(product) if !product.is_active => {
                    return Err(AppError::Validation(format!(
                        "product {product_id} is not active"
                    )));
                }
                Some(_) => {}
            }
        }

        Ok(())
    }
}

fn to_proto_balance(balance: models::stock::StockBalance) -> StockBalance {
    StockBalance {
        warehouse_id: balance.warehouse_id,
        product_id: balance.product_id,
        qty_on_hand: balance.qty_on_hand,
        qty_reserved: balance.qty_reserved,
        qty_available: balance.qty_on_hand - balance.qty_reserved,
        updated_at: wms_proto::opt_timestamp(balance.updated_at),
    }
}

fn to_proto_movement(movement: models::stock::StockMovement) -> StockMovement {
    StockMovement {
        id: movement.id,
        warehouse_id: movement.warehouse_id,
        product_id: movement.product_id,
        movement_type: movement.movement_type,
        quantity: movement.quantity,
        reference_type: movement.reference_type,
        reference_id: movement.reference_id,
        created_at: wms_proto::opt_timestamp(movement.created_at),
    }
}

#[tonic::async_trait]
impl InventoryService for InventoryGrpcService {
    #[tracing::instrument(skip_all)]
    async fn receive_stock(
        &self,
        request: Request<ReceiveStockRequest>,
    ) -> Result<Response<ReceiveStockResponse>, Status> {
        // Metadata must be read before into_inner consumes the request.
        let identity = wms_core::grpc::identity_from_metadata(request.metadata())?;
        identity.require_any_role(&RECEIVER_ROLES)?;

        let req = request.into_inner();

        if req.idempotency_key.trim().is_empty() {
            return Err(AppError::Validation("idempotency_key is required".to_string()).into());
        }
        if req.items.is_empty() {
            return Err(AppError::Validation("items must not be empty".to_string()).into());
        }
        if req.items.iter().any(|item| item.quantity <= 0) {
            return Err(AppError::Validation("quantity must be positive".to_string()).into());
        }

        // Replay check first: a retried request must return the original receipt
        // instead of adding the stock a second time.
        if let Some(existing) =
            inventory_repository::find_receipt_by_idempotency_key(&self.pool, &req.idempotency_key)
                .await?
        {
            let items = inventory_repository::list_receipt_items(&self.pool, existing.id).await?;

            return Ok(Response::new(ReceiveStockResponse {
                receipt_id: existing.id,
                warehouse_id: existing.warehouse_id,
                reference_no: existing.reference_no,
                items: items
                    .into_iter()
                    .map(|item| ReceiptItem {
                        product_id: item.product_id,
                        quantity: item.quantity,
                        unit_cost: item.unit_cost,
                    })
                    .collect(),
                created_at: wms_proto::opt_timestamp(existing.created_at),
                idempotent_replay: true,
            }));
        }

        // These two calls are what replaces the foreign keys this schema cannot
        // have. Validation happens before the transaction opens, never inside it.
        self.assert_warehouse_exists(req.warehouse_id).await?;

        let product_ids: Vec<i64> = req.items.iter().map(|item| item.product_id).collect();
        self.assert_products_exist(&product_ids).await?;

        let items: Vec<NewReceiptItem> = req
            .items
            .iter()
            .map(|item| NewReceiptItem {
                product_id: item.product_id,
                quantity: item.quantity,
                unit_cost: item.unit_cost,
            })
            .collect();

        let receipt = inventory_repository::insert_receipt(
            &self.pool,
            req.warehouse_id,
            req.reference_no.as_deref(),
            &req.idempotency_key,
            identity.user_id,
            &items,
        )
        .await?;

        tracing::info!(
            receipt_id = receipt.id,
            warehouse_id = req.warehouse_id,
            lines = items.len(),
            by = identity.user_id,
            "stock received"
        );

        Ok(Response::new(ReceiveStockResponse {
            receipt_id: receipt.id,
            warehouse_id: receipt.warehouse_id,
            reference_no: receipt.reference_no,
            items: req.items,
            created_at: wms_proto::opt_timestamp(receipt.created_at),
            idempotent_replay: false,
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn get_stock_balance(
        &self,
        request: Request<GetStockBalanceRequest>,
    ) -> Result<Response<StockBalance>, Status> {
        let req = request.into_inner();

        let balance = inventory_repository::find_balance(&self.pool, req.warehouse_id, req.product_id)
            .await?
            .ok_or_else(|| AppError::NotFound("stock balance not found".to_string()))?;

        Ok(Response::new(to_proto_balance(balance)))
    }

    #[tracing::instrument(skip_all)]
    async fn list_stock_balances(
        &self,
        request: Request<ListStockBalancesRequest>,
    ) -> Result<Response<ListStockBalancesResponse>, Status> {
        let balances =
            inventory_repository::list_balances(&self.pool, request.into_inner().warehouse_id)
                .await?;

        Ok(Response::new(ListStockBalancesResponse {
            balances: balances.into_iter().map(to_proto_balance).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn list_stock_movements(
        &self,
        request: Request<ListStockMovementsRequest>,
    ) -> Result<Response<ListStockMovementsResponse>, Status> {
        let req = request.into_inner();

        let movements =
            inventory_repository::list_movements(&self.pool, req.warehouse_id, req.product_id)
                .await?;

        Ok(Response::new(ListStockMovementsResponse {
            movements: movements.into_iter().map(to_proto_movement).collect(),
        }))
    }
}
