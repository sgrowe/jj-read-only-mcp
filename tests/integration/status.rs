use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn status() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_status", json!({})).await);
}
