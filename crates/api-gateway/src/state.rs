use std::sync::Arc;

use wms_proto::user::v1::user_service_client::UserServiceClient;
use wms_proto::inventory::v1::inventory_service_client::InventoryServiceClient;
use wms_proto::product::v1::product_service_client::ProductServiceClient;
use wms_proto::warehouse::v1::warehouse_service_client::WarehouseServiceClient;

use wms_core::grpc::TracedChannel;

use crate::config::GatewayConfig;

// The channel is cloned per request; tonic channels are cheap to clone and
// multiplex over one HTTP/2 connection.
#[derive(Clone)]
pub struct AppState {
    pub users: UserServiceClient<TracedChannel>,
    pub warehouses: WarehouseServiceClient<TracedChannel>,
    pub products: ProductServiceClient<TracedChannel>,
    pub inventory: InventoryServiceClient<TracedChannel>,
    pub config: Arc<GatewayConfig>,
}
