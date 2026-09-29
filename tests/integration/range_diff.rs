use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn diff_range() {
    let server = Server::spawn().await;
    let args = json!({"from": "ci_update@origin", "to": "main"});
    assert_snap!(server.ok("jj_diff_range", args).await);
}

#[tokio::test]
async fn diff_range_stat() {
    let server = Server::spawn().await;
    let args = json!({"from": "ci_update@origin", "to": "main", "stat": true});
    assert_snap!(server.ok("jj_diff_range", args).await);
}

#[tokio::test]
async fn interdiff() {
    let server = Server::spawn().await;
    let args = json!({"from": "ci_update@origin", "to": "main"});
    assert_snap!(server.ok("jj_interdiff", args).await);
}

#[tokio::test]
async fn interdiff_with_paths() {
    let server = Server::spawn().await;
    let args =
        json!({"from": "ci_update@origin", "to": "main", "name_only": true, "paths": [".github"]});
    assert_snap!(server.ok("jj_interdiff", args).await);
}
