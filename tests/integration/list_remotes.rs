use rmcp::serde_json::json;

use crate::common::{FIXTURE_URL, Server};

#[tokio::test]
async fn list_remotes() {
    let server = Server::spawn().await;
    assert_eq!(
        server.ok("jj_list_remotes", json!({})).await,
        format!("$ jj git remote list --ignore-working-copy\norigin {FIXTURE_URL}\n")
    );
}
