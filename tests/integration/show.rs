use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn show() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_show", json!({"revset": "main"})).await);
}

#[tokio::test]
async fn show_with_template() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "template": "commit_id ++ \"\\n\""});
    assert_snap!(server.ok("jj_show", args).await);
}
