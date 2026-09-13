use sqlx::PgPool;
use wms_core::cache::Cache;
use tonic::{Request, Response, Status};
use wms_core::AppError;
use wms_proto::product::v1::product_service_server::ProductService;
use wms_proto::product::v1::{
    Category, CreateCategoryRequest, CreateProductRequest, DeactivateProductRequest,
    GetProductRequest, GetProductsByIdsRequest, GetProductsByIdsResponse, ListCategoriesRequest,
    ListCategoriesResponse, ListProductsRequest, ListProductsResponse, Product,
    UpdateProductRequest,
};

use crate::models;
use crate::repositories::{category_repository, product_repository};

pub struct ProductGrpcService {
    pool: PgPool,
    // Option, not a hard dependency: with no REDIS_URL the service simply reads
    // the database every time and behaves identically, only slower.
    cache: Option<Cache>,
}

impl ProductGrpcService {
    pub fn new(pool: PgPool, cache: Option<Cache>) -> Self {
        Self { pool, cache }
    }

    fn cache_key(id: i64) -> String {
        format!("product:{id}")
    }

    // Read-through: ask the cache, fall back to the database, populate on the way
    // back. Products are master data - written rarely, read on every single stock
    // receipt - which is exactly the shape that caching pays off on.
    async fn load_one(&self, id: i64) -> Result<Option<models::product::Product>, AppError> {
        if let Some(cache) = &self.cache {
            if let Some(hit) = cache
                .get::<models::product::Product>(&Self::cache_key(id))
                .await
            {
                return Ok(Some(hit));
            }
        }

        let found = product_repository::find_by_id(&self.pool, id).await?;

        if let (Some(cache), Some(product)) = (&self.cache, &found) {
            cache.put(&Self::cache_key(id), product).await;
        }

        Ok(found)
    }

    // Partial hits are the normal case: serve what the cache has, fetch only the
    // remainder, then backfill.
    async fn load_many(&self, ids: &[i64]) -> Result<Vec<models::product::Product>, AppError> {
        let mut hits: Vec<models::product::Product> = Vec::new();
        let mut misses: Vec<i64> = Vec::new();

        match &self.cache {
            Some(cache) => {
                let keys: Vec<String> = ids.iter().map(|id| Self::cache_key(*id)).collect();
                let cached = cache.get_many::<models::product::Product>(&keys).await;

                for (id, entry) in ids.iter().zip(cached) {
                    match entry {
                        Some(product) => hits.push(product),
                        None => misses.push(*id),
                    }
                }
            }
            None => misses.extend_from_slice(ids),
        }

        if !misses.is_empty() {
            let fetched = product_repository::find_by_ids(&self.pool, &misses).await?;

            if let Some(cache) = &self.cache {
                for product in &fetched {
                    cache.put(&Self::cache_key(product.id), product).await;
                }
            }

            hits.extend(fetched);
        }

        tracing::debug!(
            requested = ids.len(),
            from_cache = ids.len() - misses.len(),
            from_db = misses.len(),
            "product lookup"
        );

        Ok(hits)
    }

    // Invalidate on every write. A missed invalidation here means inventory-service
    // keeps accepting receipts for a product this service already deactivated.
    async fn invalidate(&self, id: i64) {
        if let Some(cache) = &self.cache {
            cache.invalidate(&Self::cache_key(id)).await;
        }
    }
}

fn to_proto_product(product: models::product::Product) -> Product {
    Product {
        id: product.id,
        sku: product.sku,
        name: product.name,
        description: product.description,
        unit: product.unit,
        barcode: product.barcode,
        category_id: product.category_id,
        is_active: product.is_active,
        created_at: wms_proto::opt_timestamp(product.created_at),
        updated_at: wms_proto::opt_timestamp(product.updated_at),
    }
}

fn to_proto_category(category: models::category::Category) -> Category {
    Category {
        id: category.id,
        name: category.name,
        created_at: wms_proto::opt_timestamp(category.created_at),
        updated_at: wms_proto::opt_timestamp(category.updated_at),
    }
}

#[tonic::async_trait]
impl ProductService for ProductGrpcService {
    #[tracing::instrument(skip_all)]
    async fn get_product(
        &self,
        request: Request<GetProductRequest>,
    ) -> Result<Response<Product>, Status> {
        let product = self
            .load_one(request.into_inner().id)
            .await?
            .ok_or_else(|| AppError::NotFound("product not found".to_string()))?;

        Ok(Response::new(to_proto_product(product)))
    }

    #[tracing::instrument(skip_all)]
    async fn get_products_by_ids(
        &self,
        request: Request<GetProductsByIdsRequest>,
    ) -> Result<Response<GetProductsByIdsResponse>, Status> {
        let ids = request.into_inner().ids;
        let products = self.load_many(&ids).await?;

        Ok(Response::new(GetProductsByIdsResponse {
            products: products.into_iter().map(to_proto_product).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn list_products(
        &self,
        request: Request<ListProductsRequest>,
    ) -> Result<Response<ListProductsResponse>, Status> {
        let products =
            product_repository::list(&self.pool, request.into_inner().include_inactive).await?;

        Ok(Response::new(ListProductsResponse {
            products: products.into_iter().map(to_proto_product).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn create_product(
        &self,
        request: Request<CreateProductRequest>,
    ) -> Result<Response<Product>, Status> {
        let req = request.into_inner();

        if req.sku.trim().is_empty() || req.name.trim().is_empty() {
            return Err(AppError::Validation("sku and name are required".to_string()).into());
        }

        let unit = req.unit.unwrap_or_else(|| "pcs".to_string());

        let product = product_repository::create(
            &self.pool,
            &req.sku,
            &req.name,
            req.description.as_deref(),
            &unit,
            req.barcode.as_deref(),
            req.category_id,
        )
        .await?;

        Ok(Response::new(to_proto_product(product)))
    }

    #[tracing::instrument(skip_all)]
    async fn update_product(
        &self,
        request: Request<UpdateProductRequest>,
    ) -> Result<Response<Product>, Status> {
        let req = request.into_inner();

        let product = product_repository::update(
            &self.pool,
            req.id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.unit.as_deref(),
            req.barcode.as_deref(),
            req.category_id,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("product not found".to_string()))?;

        self.invalidate(product.id).await;

        Ok(Response::new(to_proto_product(product)))
    }

    #[tracing::instrument(skip_all)]
    async fn deactivate_product(
        &self,
        request: Request<DeactivateProductRequest>,
    ) -> Result<Response<()>, Status> {
        let id = request.into_inner().id;
        let affected = product_repository::deactivate(&self.pool, id).await?;

        if affected == 0 {
            return Err(AppError::NotFound("active product not found".to_string()).into());
        }

        self.invalidate(id).await;

        Ok(Response::new(()))
    }

    #[tracing::instrument(skip_all)]
    async fn list_categories(
        &self,
        _request: Request<ListCategoriesRequest>,
    ) -> Result<Response<ListCategoriesResponse>, Status> {
        let categories = category_repository::list(&self.pool).await?;

        Ok(Response::new(ListCategoriesResponse {
            categories: categories.into_iter().map(to_proto_category).collect(),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn create_category(
        &self,
        request: Request<CreateCategoryRequest>,
    ) -> Result<Response<Category>, Status> {
        let req = request.into_inner();

        if req.name.trim().is_empty() {
            return Err(AppError::Validation("name is required".to_string()).into());
        }

        let category = category_repository::create(&self.pool, &req.name).await?;

        Ok(Response::new(to_proto_category(category)))
    }
}
