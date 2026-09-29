use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn diff() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "paths": ["Cargo.toml", "Cargo.lock"]});
    assert_snap!(server.ok("jj_diff", args).await);
}

#[tokio::test]
async fn diff_context() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "paths": [".github/workflows/ci.yml"], "context": 1});
    assert_snap!(server.ok("jj_diff", args).await);
}

#[tokio::test]
async fn diff_stat() {
    let server = Server::spawn().await;
    assert_snap!(
        server
            .ok("jj_diff", json!({"revset": "main", "stat": true}))
            .await
    );
}

#[tokio::test]
async fn diff_summary() {
    let server = Server::spawn().await;
    assert_snap!(
        server
            .ok("jj_diff", json!({"revset": "main", "summary": true}))
            .await
    );
}

#[tokio::test]
async fn diff_name_only() {
    let server = Server::spawn().await;
    assert_snap!(
        server
            .ok("jj_diff", json!({"revset": "main", "name_only": true}))
            .await
    );
}

#[tokio::test]
async fn diff_conflicting_formats() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "stat": true, "summary": true});
    let text = server.err("jj_diff", args).await;
    assert!(text.ends_with("exit code: 2"), "{text}");
    assert_snap!(text);
}
