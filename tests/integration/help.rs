use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn help() {
    let server = Server::spawn().await;
    let text = server.ok("jj_help", json!({"topic": "revsets"})).await;
    let first_line = text.lines().next().unwrap();
    assert_eq!(
        first_line,
        "$ jj help --ignore-working-copy --keyword=revsets"
    );
    assert!(text.contains("String patterns"), "{text}");
}

#[tokio::test]
async fn help_bad_topic() {
    let server = Server::spawn().await;
    assert_snap!(server.err("jj_help", json!({"topic": "nope"})).await);
}
