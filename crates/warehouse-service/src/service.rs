use sqlx::PgPool;
use tonic::{Request, Response, Status};
use wms_core::AppError;
use wms_proto::warehouse::v1::warehouse_service_server::WarehouseService;
use wms_proto::warehouse::v1::{
    CreateWarehouseRequest, DeleteWarehouseRequest, GetWarehouseRequest, ListWarehousesRequest,
    ListWarehousesResponse, UpdateWarehouseRequest, Warehouse,
};

use crate::models;
use crate::repositories::warehouse_repository;

pub struct WarehouseGrpcService {
    pool: PgPool,
}

impl WarehouseGrpcService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn to_proto(warehouse: models::warehouse::Warehouse) -> Warehouse {
    Warehouse {
        id: warehouse.id,
        name: warehouse.name,
        address: warehouse.address,
        phone: warehouse.phone,
        photo: warehouse.photo,
        created_at: wms_proto::opt_timestamp(warehouse.created_at),
        updated_at: wms_proto::opt_timestamp(warehouse.updated_at),
    }
}

#[tonic::async_trait]
impl WarehouseService for WarehouseGrpcService {
    async fn get_warehouse(
        &self,
        request: Request<GetWarehouseRequest>,
    ) -> Result<Response<Warehouse>, Status> {
        let warehouse = warehouse_repository::find_by_id(&self.pool, request.into_inner().id)
            .await?
            .ok_or_else(|| AppError::NotFound("warehouse not found".to_string()))?;

        Ok(Response::new(to_proto(warehouse)))
    }

    async fn list_warehouses(
        &self,
        _request: Request<ListWarehousesRequest>,
    ) -> Result<Response<ListWarehousesResponse>, Status> {
        let warehouses = warehouse_repository::list(&self.pool).await?;

        Ok(Response::new(ListWarehousesResponse {
            warehouses: warehouses.into_iter().map(to_proto).collect(),
        }))
    }

    async fn create_warehouse(
        &self,
        request: Request<CreateWarehouseRequest>,
    ) -> Result<Response<Warehouse>, Status> {
        let req = request.into_inner();

        if req.name.trim().is_empty() || req.address.trim().is_empty() {
            return Err(
                AppError::Validation("name and address are required".to_string()).into(),
            );
        }

        let warehouse = warehouse_repository::create(
            &self.pool,
            &req.name,
            &req.address,
            req.phone.as_deref(),
            req.photo.as_deref(),
        )
        .await?;

        Ok(Response::new(to_proto(warehouse)))
    }

    async fn update_warehouse(
        &self,
        request: Request<UpdateWarehouseRequest>,
    ) -> Result<Response<Warehouse>, Status> {
        let req = request.into_inner();

        let warehouse = warehouse_repository::update(
            &self.pool,
            req.id,
            req.name.as_deref(),
            req.address.as_deref(),
            req.phone.as_deref(),
            req.photo.as_deref(),
        )
        .await?
        .ok_or_else(|| AppError::NotFound("warehouse not found".to_string()))?;

        Ok(Response::new(to_proto(warehouse)))
    }

    async fn delete_warehouse(
        &self,
        request: Request<DeleteWarehouseRequest>,
    ) -> Result<Response<()>, Status> {
        let affected =
            warehouse_repository::soft_delete(&self.pool, request.into_inner().id).await?;

        if affected == 0 {
            return Err(AppError::NotFound("warehouse not found".to_string()).into());
        }

        Ok(Response::new(()))
    }
}
