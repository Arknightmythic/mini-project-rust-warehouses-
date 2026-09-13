use sqlx::PgPool;
use wms_core::cache::Cache;
use tonic::{Request, Response, Status};
use wms_core::AppError;
use wms_proto::warehouse::v1::warehouse_service_server::WarehouseService;
use wms_proto::warehouse::v1::{
    CreateWarehouseRequest, DeleteWarehouseRequest, GetWarehouseRequest, ListWarehousesRequest,
    ListWarehousesResponse, UpdateWarehouseRequest, Warehouse, WarehouseExistsRequest,
    WarehouseExistsResponse,
};

use crate::models;
use crate::repositories::warehouse_repository;

pub struct WarehouseGrpcService {
    pool: PgPool,
    cache: Option<Cache>,
}

impl WarehouseGrpcService {
    pub fn new(pool: PgPool, cache: Option<Cache>) -> Self {
        Self { pool, cache }
    }

    fn cache_key(id: i64) -> String {
        format!("warehouse:name:{id}")
    }

    // Only positive results are cached. Caching "does not exist" would mean a
    // freshly created warehouse looks missing until the TTL expires, which is a
    // worse trade than the occasional wasted lookup for a bad id.
    async fn load_name(&self, id: i64) -> Result<Option<String>, AppError> {
        if let Some(cache) = &self.cache {
            if let Some(name) = cache.get::<String>(&Self::cache_key(id)).await {
                return Ok(Some(name));
            }
        }

        let found = warehouse_repository::exists(&self.pool, id).await?;

        if let (Some(cache), Some(name)) = (&self.cache, &found) {
            cache.put(&Self::cache_key(id), name).await;
        }

        Ok(found)
    }

    async fn invalidate(&self, id: i64) {
        if let Some(cache) = &self.cache {
            cache.invalidate(&Self::cache_key(id)).await;
        }
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
    #[tracing::instrument(skip_all)]
    async fn get_warehouse(
        &self,
        request: Request<GetWarehouseRequest>,
    ) -> Result<Response<Warehouse>, Status> {
        let warehouse = warehouse_repository::find_by_id(&self.pool, request.into_inner().id)
            .await?
            .ok_or_else(|| AppError::NotFound("warehouse not found".to_string()))?;

        Ok(Response::new(to_proto(warehouse)))
    }

    #[tracing::instrument(skip_all)]
    async fn list_warehouses(
        &self,
        _request: Request<ListWarehousesRequest>,
    ) -> Result<Response<ListWarehousesResponse>, Status> {
        let warehouses = warehouse_repository::list(&self.pool).await?;

        Ok(Response::new(ListWarehousesResponse {
            warehouses: warehouses.into_iter().map(to_proto).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
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

    #[tracing::instrument(skip_all)]
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

        self.invalidate(warehouse.id).await;

        Ok(Response::new(to_proto(warehouse)))
    }

    #[tracing::instrument(skip_all)]
    async fn warehouse_exists(
        &self,
        request: Request<WarehouseExistsRequest>,
    ) -> Result<Response<WarehouseExistsResponse>, Status> {
        let found = self.load_name(request.into_inner().id).await?;

        Ok(Response::new(WarehouseExistsResponse {
            exists: found.is_some(),
            name: found.unwrap_or_default(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn delete_warehouse(
        &self,
        request: Request<DeleteWarehouseRequest>,
    ) -> Result<Response<()>, Status> {
        let id = request.into_inner().id;
        let affected = warehouse_repository::soft_delete(&self.pool, id).await?;

        if affected == 0 {
            return Err(AppError::NotFound("warehouse not found".to_string()).into());
        }

        // Without this, inventory would keep accepting receipts into a warehouse
        // that was just deleted, for as long as the TTL lasts.
        self.invalidate(id).await;

        Ok(Response::new(()))
    }
}
