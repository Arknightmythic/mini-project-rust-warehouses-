use tonic::{Request, Response, Status};
use wms_core::AppError;
use wms_events::{Envelope, ReceivedLine, StockReservationReleased, StockReserved, StockShipped};
use wms_proto::inventory::v1::{
    CancelShipmentRequest, ConfirmShipmentRequest, GetShipmentRequest, ListShipmentsRequest,
    ListShipmentsResponse, Shipment, ShipStockRequest, ShipmentItem,
};
use wms_proto::product::v1 as product_v1;

use crate::models;
use crate::repositories::saga_repository::{self, NewShipmentLine};
use crate::service::InventoryGrpcService;

pub const SHIPPER_ROLES: [&str; 2] = ["admin", "warehouse_manager"];

pub fn to_proto(shipment: models::stock::OutboundShipment, lines: Vec<ShipmentItem>) -> Shipment {
    Shipment {
        id: shipment.id,
        warehouse_id: shipment.warehouse_id,
        reference_no: shipment.reference_no,
        status: shipment.status,
        items: lines,
        created_at: wms_proto::opt_timestamp(shipment.created_at),
        updated_at: wms_proto::opt_timestamp(shipment.updated_at),
    }
}

impl InventoryGrpcService {
    pub(crate) async fn load_shipment(&self, id: i64) -> Result<Shipment, AppError> {
        let shipment = saga_repository::find(self.pool(), id)
            .await?
            .ok_or_else(|| AppError::NotFound("shipment not found".to_string()))?;

        let lines = saga_repository::list_lines(self.pool(), id)
            .await?
            .into_iter()
            .map(|line| ShipmentItem {
                product_id: line.product_id,
                quantity: line.quantity,
            })
            .collect();

        Ok(to_proto(shipment, lines))
    }

    pub(crate) async fn saga_ship_stock(
        &self,
        request: Request<ShipStockRequest>,
    ) -> Result<Response<Shipment>, Status> {
        let identity = self.caller(request.metadata())?;
        identity.require_any_role(&SHIPPER_ROLES)?;

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

        if let Some(existing) =
            saga_repository::find_by_idempotency_key(self.pool(), &req.idempotency_key).await?
        {
            return Ok(Response::new(self.load_shipment(existing.id).await?));
        }

        self.assert_warehouse_exists(req.warehouse_id).await?;

        let product_ids: Vec<i64> = req.items.iter().map(|item| item.product_id).collect();
        self.assert_products_exist(&product_ids).await?;

        let lines: Vec<NewShipmentLine> = req
            .items
            .iter()
            .map(|item| NewShipmentLine {
                product_id: item.product_id,
                quantity: item.quantity,
            })
            .collect();

        let shipment = saga_repository::reserve(
            self.pool(),
            req.warehouse_id,
            req.reference_no.as_deref(),
            &req.idempotency_key,
            identity.user_id,
            &lines,
        )
        .await?;

        self.publish(
            wms_events::topology::ROUTING_STOCK_RESERVED,
            Envelope::new(
                "inventory.stock.reserved",
                StockReserved {
                    shipment_id: shipment.id,
                    warehouse_id: shipment.warehouse_id,
                    reference_no: shipment.reference_no.clone(),
                    reserved_by: identity.user_id,
                    lines: to_event_lines(&req.items),
                },
            ),
        )
        .await;

        tracing::info!(shipment_id = shipment.id, "stock reserved");

        Ok(Response::new(self.load_shipment(shipment.id).await?))
    }

    pub(crate) async fn saga_confirm_shipment(
        &self,
        request: Request<ConfirmShipmentRequest>,
    ) -> Result<Response<Shipment>, Status> {
        let identity = self.caller(request.metadata())?;
        identity.require_any_role(&SHIPPER_ROLES)?;

        let shipment_id = request.into_inner().shipment_id;

        let shipment = saga_repository::find(self.pool(), shipment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("shipment not found".to_string()))?;

        if shipment.status != "RESERVED" {
            return Err(AppError::Conflict(format!(
                "shipment {shipment_id} is {} and can no longer be confirmed",
                shipment.status
            ))
            .into());
        }

        let lines = saga_repository::list_lines(self.pool(), shipment_id).await?;

        // The step that can fail for an external reason. If product-service is
        // unreachable this returns 503 and the shipment stays RESERVED - which is
        // exactly why the intermediate state had to be persisted.
        let mut products = self.products();
        let catalog = products
            .get_products_by_ids(Request::new(product_v1::GetProductsByIdsRequest {
                ids: lines.iter().map(|line| line.product_id).collect(),
            }))
            .await
            .map_err(AppError::from)?
            .into_inner()
            .products;

        for line in &lines {
            match catalog.iter().find(|p| p.id == line.product_id) {
                Some(product) if product.is_active => {}
                _ => {
                    return Err(AppError::Validation(format!(
                        "product {} is no longer shippable",
                        line.product_id
                    ))
                    .into());
                }
            }
        }

        if !saga_repository::confirm(self.pool(), shipment_id).await? {
            return Err(AppError::Conflict("shipment was already resolved".to_string()).into());
        }

        self.publish(
            wms_events::topology::ROUTING_STOCK_SHIPPED,
            Envelope::new(
                "inventory.stock.shipped",
                StockShipped {
                    shipment_id,
                    warehouse_id: shipment.warehouse_id,
                    reference_no: shipment.reference_no.clone(),
                    lines: lines
                        .iter()
                        .map(|line| ReceivedLine {
                            product_id: line.product_id,
                            quantity: line.quantity,
                        })
                        .collect(),
                },
            ),
        )
        .await;

        tracing::info!(shipment_id, "shipment confirmed");

        Ok(Response::new(self.load_shipment(shipment_id).await?))
    }

    pub(crate) async fn saga_cancel_shipment(
        &self,
        request: Request<CancelShipmentRequest>,
    ) -> Result<Response<Shipment>, Status> {
        let identity = self.caller(request.metadata())?;
        identity.require_any_role(&SHIPPER_ROLES)?;

        let req = request.into_inner();
        let reason = req.reason.unwrap_or_else(|| "cancelled by operator".to_string());

        self.compensate(req.shipment_id, &reason).await?;

        Ok(Response::new(self.load_shipment(req.shipment_id).await?))
    }

    // Shared by the manual cancel RPC and by the sweeper, because compensation is
    // the same operation regardless of who noticed the shipment was stuck.
    pub(crate) async fn compensate(&self, shipment_id: i64, reason: &str) -> Result<(), AppError> {
        let shipment = saga_repository::find(self.pool(), shipment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("shipment not found".to_string()))?;

        let lines = saga_repository::list_lines(self.pool(), shipment_id).await?;

        if !saga_repository::release(self.pool(), shipment_id).await? {
            return Err(AppError::Conflict(format!(
                "shipment {shipment_id} is {} and cannot be cancelled",
                shipment.status
            )));
        }

        self.publish(
            wms_events::topology::ROUTING_STOCK_RELEASED,
            Envelope::new(
                "inventory.stock.released",
                StockReservationReleased {
                    shipment_id,
                    warehouse_id: shipment.warehouse_id,
                    reason: reason.to_string(),
                    lines: lines
                        .iter()
                        .map(|line| ReceivedLine {
                            product_id: line.product_id,
                            quantity: line.quantity,
                        })
                        .collect(),
                },
            ),
        )
        .await;

        tracing::info!(shipment_id, reason, "reservation released");

        Ok(())
    }

    pub(crate) async fn saga_get_shipment(
        &self,
        request: Request<GetShipmentRequest>,
    ) -> Result<Response<Shipment>, Status> {
        Ok(Response::new(
            self.load_shipment(request.into_inner().shipment_id).await?,
        ))
    }

    pub(crate) async fn saga_list_shipments(
        &self,
        _request: Request<ListShipmentsRequest>,
    ) -> Result<Response<ListShipmentsResponse>, Status> {
        let stale = saga_repository::find_stale(self.pool(), 0).await?;

        let mut shipments = Vec::new();
        for id in stale {
            shipments.push(self.load_shipment(id).await?);
        }

        Ok(Response::new(ListShipmentsResponse { shipments }))
    }
}

fn to_event_lines(items: &[ShipmentItem]) -> Vec<ReceivedLine> {
    items
        .iter()
        .map(|item| ReceivedLine {
            product_id: item.product_id,
            quantity: item.quantity,
        })
        .collect()
}
