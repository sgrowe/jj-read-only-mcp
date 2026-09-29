use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn bookmark_list() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_bookmark_list", json!({})).await);
}

#[tokio::test]
async fn bookmark_list_all_remotes() {
    let server = Server::spawn().await;
    let args = json!({"all_remotes": true, "names": ["ci*"]});
    assert_snap!(server.ok("jj_bookmark_list", args).await);
}

#[tokio::test]
async fn bookmark_list_revset() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "template": "name ++ \"\\n\""});
    assert_snap!(server.ok("jj_bookmark_list", args).await);
}

#[tokio::test]
async fn bookmark_list_ignores_default() {
    let server = Server::spawn().await;
    server
        .ok(
            "jj_set_default_revision",
            json!({"revision": "ci_update@origin"}),
        )
        .await;
    let text = server.ok("jj_bookmark_list", json!({})).await;
    let first_line = text.lines().next().unwrap();
    assert_eq!(first_line, "$ jj bookmark list --ignore-working-copy");
}
