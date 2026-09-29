use rmcp::serde_json::json;

use crate::common::{Server, assert_snap, jj, repo, server_command};

#[tokio::test]
async fn lists_tools() {
    let server = Server::spawn().await;
    assert_snap!(server.tools_json().await);
}

/// Approximate token cost of what the model sees: the instructions, and each
/// tool's name, description and input schema (not its annotations).
#[tokio::test]
async fn token_counts() {
    use std::fmt::Write as _;

    let server = Server::spawn().await;
    let bpe = tiktoken_rs::o200k_base_singleton();
    let count = |text: &str| bpe.encode_ordinary(text).len();

    // (name, description, schema, total)
    let instructions = count(&server.instructions());
    let mut rows = vec![("instructions".to_owned(), instructions, None, instructions)];
    for tool in server.tools().await {
        let description = tool.description.as_deref().unwrap_or_default();
        let schema = rmcp::serde_json::to_string(&tool.input_schema).unwrap();
        let definition = json!({
            "name": &tool.name,
            "description": description,
            "inputSchema": &tool.input_schema,
        });
        rows.push((
            tool.name.to_string(),
            count(description),
            Some(count(&schema)),
            count(&definition.to_string()),
        ));
    }

    let width = rows.iter().map(|row| row.0.len()).max().unwrap();
    let mut table = String::from(
        "Approximate tokens (tiktoken o200k_base). Claude's tokenizer differs,\n\
         so use these to compare changes, not as exact costs.\n\n",
    );
    writeln!(table, "{:width$}  description  schema  total", "").unwrap();
    for (name, description, schema, total) in &rows {
        let schema = schema.map_or_else(|| "-".to_owned(), |schema| schema.to_string());
        writeln!(
            table,
            "{name:width$}  {description:>11}  {schema:>6}  {total:>5}"
        )
        .unwrap();
    }
    let total: usize = rows.iter().map(|row| row.3).sum();
    writeln!(table, "{:width$}  {total:>26}", "total").unwrap();
    assert_snap!(table);
}

#[tokio::test]
async fn no_output() {
    let server = Server::spawn().await;
    let text = server
        .ok("jj_diff", json!({"revset": "main", "paths": ["src"]}))
        .await;
    assert_eq!(
        text,
        "$ jj diff --ignore-working-copy --revisions=main --git -- src\n(no output)"
    );
}

/// `--color=never` is passed but not echoed. It wins over user config that
/// forces colour on.
#[tokio::test]
async fn colour_is_off_even_if_config_forces_it() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    std::fs::write(&config, "ui.color = \"always\"\n").unwrap();
    let mut command = server_command(repo());
    command.env("JJ_CONFIG", &config);
    let server = Server::spawn_command(command).await;

    let text = server.ok("jj_log", json!({"revset": "main"})).await;
    assert!(!text.contains('\x1b'), "{text:?}");
    let first_line = text.lines().next().unwrap();
    assert!(!first_line.contains("--color"), "{first_line}");
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
        ("jj_list_remotes", json!({})),
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
