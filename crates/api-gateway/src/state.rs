use std::sync::Arc;

use wms_proto::user::v1::user_service_client::UserServiceClient;

use crate::config::GatewayConfig;

// The channel is cloned per request; tonic channels are cheap to clone and
// multiplex over one HTTP/2 connection.
#[derive(Clone)]
pub struct AppState {
    pub users: UserServiceClient<tonic::transport::Channel>,
    pub config: Arc<GatewayConfig>,
}
