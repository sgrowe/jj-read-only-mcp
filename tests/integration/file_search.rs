use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn file_search() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "pattern": "Tera", "paths": ["src"]});
    assert_snap!(server.ok("jj_file_search", args).await);
}

#[tokio::test]
async fn file_search_name_only() {
    let server = Server::spawn().await;
    let args = json!({
        "revision": "main",
        "pattern": "TERA",
        "kind": "substring",
        "ignore_case": true,
        "name_only": true,
    });
    assert_snap!(server.ok("jj_file_search", args).await);
}

/// A colon in the pattern isn't read as a pattern kind.
#[tokio::test]
async fn file_search_pattern_with_colon() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "pattern": "tera::Result", "paths": ["src"]});
    assert_snap!(server.ok("jj_file_search", args).await);
}
