use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn evolog() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_evolog", json!({"revset": "main"})).await);
}

#[tokio::test]
async fn evolog_with_template() {
    let server = Server::spawn().await;
    let args = json!({
        "revset": "main",
        "no_graph": true,
        "limit": 1,
        "template": "commit.commit_id() ++ \"\\n\"",
    });
    assert_snap!(server.ok("jj_evolog", args).await);
}
