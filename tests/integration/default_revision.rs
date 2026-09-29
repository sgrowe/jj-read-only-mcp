use rmcp::serde_json::json;

use crate::common::{Server, assert_snap};

#[tokio::test]
async fn missing_revision_without_default() {
    let server = Server::spawn().await;
    let text = server.err("jj_file_list", json!({})).await;
    assert!(!text.starts_with("$ "), "ran jj:\n{text}");
    assert_snap!(text);
}

#[tokio::test]
async fn default_revision_fills_omitted_params() {
    let server = Server::spawn().await;
    assert_snap!(
        server
            .ok("jj_set_default_revision", json!({"revision": "main"}))
            .await
    );
    assert_eq!(
        server.ok("jj_file_list", json!({})).await,
        server.ok("jj_file_list", json!({"revision": "main"})).await
    );
    assert_eq!(
        server.ok("jj_show", json!({})).await,
        server.ok("jj_show", json!({"revset": "main"})).await
    );
    // A given param still wins over the default.
    let text = server
        .ok("jj_file_list", json!({"revision": "ci_update@origin"}))
        .await;
    assert!(text.starts_with("$ jj file list --ignore-working-copy --revision=ci_update@origin\n"));
}

#[tokio::test]
async fn bad_default_revision_is_not_stored() {
    let server = Server::spawn().await;
    let text = server
        .err("jj_set_default_revision", json!({"revision": "bad(("}))
        .await;
    assert!(text.ends_with("exit code: 1"), "{text}");
    server.err("jj_file_list", json!({})).await;

    // A bad value doesn't replace a good one either.
    server
        .ok("jj_set_default_revision", json!({"revision": "main"}))
        .await;
    server
        .err("jj_set_default_revision", json!({"revision": "bad(("}))
        .await;
    server.ok("jj_file_list", json!({})).await;
}

#[tokio::test]
async fn clear_default_revision() {
    let server = Server::spawn().await;
    for cleared in [
        json!({"revision": null}),
        json!({"revision": ""}),
        json!({}),
    ] {
        server
            .ok("jj_set_default_revision", json!({"revision": "main"}))
            .await;
        let text = server.ok("jj_set_default_revision", cleared).await;
        assert_eq!(text, "Default revision cleared.");
        server.err("jj_file_list", json!({})).await;
    }
}
