use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn file_list() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_file_list", json!({"revision": "main"})).await);
}

#[tokio::test]
async fn file_list_with_paths() {
    let server = Server::spawn().await;
    let args = json!({"revision": "ci_update@origin", "paths": ["src/fixtures"]});
    assert_snap!(server.ok("jj_file_list", args).await);
}
