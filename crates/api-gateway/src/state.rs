use std::sync::Arc;

use wms_proto::user::v1::user_service_client::UserServiceClient;
use wms_proto::product::v1::product_service_client::ProductServiceClient;
use wms_proto::warehouse::v1::warehouse_service_client::WarehouseServiceClient;

use crate::config::GatewayConfig;

// The channel is cloned per request; tonic channels are cheap to clone and
// multiplex over one HTTP/2 connection.
#[derive(Clone)]
pub struct AppState {
    pub users: UserServiceClient<tonic::transport::Channel>,
    pub warehouses: WarehouseServiceClient<tonic::transport::Channel>,
    pub products: ProductServiceClient<tonic::transport::Channel>,
    pub config: Arc<GatewayConfig>,
}
