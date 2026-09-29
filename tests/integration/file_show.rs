use rmcp::serde_json::json;

use crate::common::{Server, assert_snap, jj};

#[tokio::test]
async fn file_show() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "paths": ["src/str.rs", "conventions.md"]});
    assert_snap!(server.ok("jj_file_show", args).await);
}

#[tokio::test]
async fn file_show_with_template() {
    let server = Server::spawn().await;
    let args = json!({
        "revision": "main",
        "paths": ["src/str.rs", "conventions.md"],
        "template": "path ++ \"\\n\"",
    });
    assert_snap!(server.ok("jj_file_show", args).await);
}

/// One path gets no header template, so the user's `templates.file_show`
/// applies. A given `template` is still passed.
#[tokio::test]
async fn file_show_one_path() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "paths": ["conventions.md"]});
    let text = server.ok("jj_file_show", args).await;
    assert_eq!(
        text.lines().next().unwrap(),
        "$ jj file show --ignore-working-copy --revision=main -- conventions.md"
    );
    assert!(!text.contains("==>"), "{text}");
    assert_snap!(text);

    let args = json!({
        "revision": "main",
        "paths": ["conventions.md"],
        "template": "path ++ \"\\n\"",
    });
    let text = server.ok("jj_file_show", args).await;
    assert_eq!(
        text.lines().next().unwrap(),
        "$ jj file show --ignore-working-copy --revision=main \
         '--template=path ++ \"\\n\"' -- conventions.md"
    );
}

/// The header starts with a newline, so it isn't joined to the last line of a
/// file with no final newline.
#[tokio::test]
async fn file_show_header_after_no_final_newline() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    jj(dir, &["git", "init"]);
    std::fs::write(dir.join("a.txt"), "x").unwrap();
    std::fs::write(dir.join("b.txt"), "y\n").unwrap();
    jj(dir, &["commit", "--message=files"]);

    let server = Server::spawn_in(dir).await;
    let args = json!({"revision": "@-", "paths": ["a.txt", "b.txt"]});
    assert_eq!(
        server.ok("jj_file_show", args).await,
        "$ jj file show --ignore-working-copy --revision=@- \
         '--template=\"\\n==> \" ++ format_path(path) ++ \" <==\\n\"' -- a.txt b.txt\n\
         \n==> a.txt <==\nx\n==> b.txt <==\ny\n"
    );
}
