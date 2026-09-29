use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn file_annotate() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "path": "src/str.rs"});
    assert_snap!(server.ok("jj_file_annotate", args).await);
}
