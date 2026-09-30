use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn log() {
    let server = Server::spawn().await;
    assert_snap!(
        server
            .ok("jj_log", json!({"revset": "::main", "limit": 3}))
            .await
    );
}

#[tokio::test]
async fn log_default_revset() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_log", json!({})).await);
}

#[tokio::test]
async fn log_patch() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "no_graph": true, "patch": true});
    assert_snap!(server.ok("jj_log", args).await);
}

#[tokio::test]
async fn log_summary_with_template() {
    let server = Server::spawn().await;
    let args = json!({
        "revset": "::main",
        "limit": 3,
        "reversed": true,
        "summary": true,
        "template": "description.first_line() ++ \"\\n\"",
    });
    assert_snap!(server.ok("jj_log", args).await);
}

#[tokio::test]
async fn log_stat() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "no_graph": true, "stat": true});
    assert_snap!(server.ok("jj_log", args).await);
}
