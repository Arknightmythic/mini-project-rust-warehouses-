// A hand-written gRPC client, so the service can be exercised without the gateway
// and without installing grpcurl. Run: cargo run -p user-service --example ping
use wms_proto::user::v1::ListRolesRequest;
use wms_proto::user::v1::user_service_client::UserServiceClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:50051".to_string());

    let mut client = UserServiceClient::connect(url.clone()).await?;
    let response = client.list_roles(ListRolesRequest {}).await?;

    println!("connected to {url}");
    for role in response.into_inner().roles {
        println!("  role {:>2} = {}", role.id, role.name);
    }

    Ok(())
}
