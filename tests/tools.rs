//! Runs each tool against a clone of the fixture repo. Uses `main` (the same
//! commit as tag `v0.1.5`) and `ci_update@origin`, never `@`. Tests that need
//! their own repo make one in a temp dir.

#[macro_use]
mod common;

use common::{Server, jj, server_command};
use rmcp::serde_json::json;

#[tokio::test]
async fn lists_tools() {
    let server = Server::spawn().await;
    assert_snap!(server.tools_json().await);
}

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

#[tokio::test]
async fn show() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_show", json!({"revset": "main"})).await);
}

#[tokio::test]
async fn show_with_template() {
    let server = Server::spawn().await;
    let args = json!({"revset": "main", "template": "commit_id ++ \"\\n\""});
    assert_snap!(server.ok("jj_show", args).await);
}

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

#[tokio::test]
async fn no_output() {
    let server = Server::spawn().await;
    let text = server
        .ok("jj_diff", json!({"revset": "main", "paths": ["src"]}))
        .await;
    assert_eq!(
        text,
        "$ jj diff --ignore-working-copy --color=never --revisions=main --git -- src\n(no output)"
    );
}

#[tokio::test]
async fn evolog() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_evolog", json!({"revset": "main"})).await);
}

#[tokio::test]
async fn evolog_with_template() {
    let server = Server::spawn().await;
    let args = json!({
        "revset": "main",
        "no_graph": true,
        "limit": 1,
        "template": "commit.commit_id() ++ \"\\n\"",
    });
    assert_snap!(server.ok("jj_evolog", args).await);
}

#[tokio::test]
async fn file_show() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "paths": ["src/str.rs", "conventions.md"]});
    assert_snap!(server.ok("jj_file_show", args).await);
}

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

#[tokio::test]
async fn file_search() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "pattern": "Tera", "paths": ["src"]});
    assert_snap!(server.ok("jj_file_search", args).await);
}

#[tokio::test]
async fn file_search_name_only() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "pattern": "substring:tera", "name_only": true});
    assert_snap!(server.ok("jj_file_search", args).await);
}

#[tokio::test]
async fn file_annotate() {
    let server = Server::spawn().await;
    let args = json!({"revision": "main", "path": "src/str.rs"});
    assert_snap!(server.ok("jj_file_annotate", args).await);
}

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
    assert_eq!(
        first_line,
        "$ jj bookmark list --ignore-working-copy --color=never"
    );
}

#[tokio::test]
async fn help() {
    let server = Server::spawn().await;
    let text = server.ok("jj_help", json!({"topic": "revsets"})).await;
    let first_line = text.lines().next().unwrap();
    assert_eq!(
        first_line,
        "$ jj help --ignore-working-copy --color=never --keyword=revsets"
    );
    assert!(text.contains("String patterns"), "{text}");
}

#[tokio::test]
async fn help_bad_topic() {
    let server = Server::spawn().await;
    assert_snap!(server.err("jj_help", json!({"topic": "nope"})).await);
}

#[tokio::test]
async fn status() {
    let server = Server::spawn().await;
    assert_snap!(server.ok("jj_status", json!({})).await);
}

#[tokio::test]
async fn bad_revset() {
    let server = Server::spawn().await;
    let text = server.err("jj_log", json!({"revset": "main(("})).await;
    assert!(text.ends_with("exit code: 1"), "{text}");
    assert_snap!(text);
}

#[tokio::test]
async fn flag_like_values_are_not_parsed_as_flags() {
    let server = Server::spawn().await;
    for (tool, args) in [
        ("jj_log", json!({"revset": "--version"})),
        ("jj_show", json!({"revset": "--version"})),
        (
            "jj_file_show",
            json!({"revision": "main", "paths": ["--version"]}),
        ),
    ] {
        let text = server.err(tool, args).await;
        assert!(
            !text.contains("jj 0."),
            "{tool} printed the version:\n{text}"
        );
    }
    assert_snap!(server.err("jj_show", json!({"revset": "--version"})).await);
}

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
    assert!(text.starts_with(
        "$ jj file list --ignore-working-copy --color=never --revision=ci_update@origin\n"
    ));
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

#[tokio::test]
async fn exits_outside_a_repo() {
    // Not `CARGO_TARGET_TMPDIR`: that is inside this project's jj repo.
    let dir = tempfile::tempdir().unwrap();
    let output = server_command(dir.path()).output().await.unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("There is no jj repo"), "{stderr}");
}

#[tokio::test]
async fn unknown_params_are_rejected() {
    let server = Server::spawn().await;
    server
        .ok("jj_set_default_revision", json!({"revision": "main"}))
        .await;
    // `jj_show` takes `revset`, not `revision`. This must not show the default.
    let text = server
        .err("jj_show", json!({"revision": "ci_update@origin"}))
        .await;
    assert!(text.contains("unknown field `revision`"), "{text}");
}

#[tokio::test]
async fn does_not_write_to_the_repo() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    jj(dir, &["git", "init", "--colocate"]);
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    jj(dir, &["commit", "--message=one"]);
    jj(dir, &["bookmark", "create", "--revision=@-", "b"]);
    // Leave a change in the working copy, so a snapshot would be a new op.
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    let op_log = || {
        jj(
            dir,
            &[
                "op",
                "log",
                "--ignore-working-copy",
                "--template=id ++ \"\\n\"",
            ],
        )
    };
    let before = op_log();

    let server = Server::spawn_in(dir).await;
    for (tool, args) in [
        ("jj_set_default_revision", json!({"revision": "@-"})),
        ("jj_log", json!({"patch": true})),
        ("jj_show", json!({})),
        ("jj_diff", json!({})),
        ("jj_diff_range", json!({"from": "root()", "to": "@"})),
        ("jj_interdiff", json!({"from": "@-", "to": "@"})),
        ("jj_evolog", json!({"patch": true})),
        ("jj_file_show", json!({"paths": ["a.txt"]})),
        ("jj_file_list", json!({})),
        ("jj_file_search", json!({"pattern": "one"})),
        ("jj_file_annotate", json!({"path": "a.txt"})),
        ("jj_bookmark_list", json!({"all_remotes": true})),
        ("jj_help", json!({"topic": "revsets"})),
    ] {
        server.ok(tool, args).await;
    }
    let status = server.ok("jj_status", json!({})).await;
    assert!(
        status.contains("The working copy has no changes."),
        "{status}"
    );

    assert_eq!(op_log(), before);
}
