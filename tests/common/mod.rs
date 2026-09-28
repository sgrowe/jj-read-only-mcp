//! Test harness: the fixture repo, and an MCP client talking to the server
//! binary over stdio.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use rmcp::model::CallToolRequestParams;
use rmcp::serde_json::Value;
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};

const FIXTURE_URL: &str = "https://github.com/sgrowe/aider-script";

/// Snapshots `$text`, redacting the working-copy IDs in `jj status` output
/// (they are random for each clone).
macro_rules! assert_snap {
    ($text:expr) => {{
        // Evaluated first: `with_settings!` runs its body in a closure, so
        // `$text` can't `.await` inside it.
        let text = $text;
        insta::with_settings!({
            filters => vec![(
                r"(Working copy  \(@\) : )[a-z]+ [0-9a-f]+",
                "$1[change-id] [commit-id]",
            )],
        }, {
            insta::assert_snapshot!(text);
        })
    }};
}

fn tmp_dir() -> &'static Path {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
}

/// An empty jj config, so the user's config can't change the output.
fn empty_config() -> PathBuf {
    tmp_dir().join("empty-jj-config.toml")
}

/// The fixture repo, cloned once per test run.
fn repo() -> &'static Path {
    static REPO: OnceLock<PathBuf> = OnceLock::new();
    REPO.get_or_init(|| {
        std::fs::write(empty_config(), "").unwrap();
        let repo = tmp_dir().join("aider-script");
        if repo.exists() {
            std::fs::remove_dir_all(&repo).unwrap();
        }
        let output = Command::new("jj")
            .args(["git", "clone", FIXTURE_URL])
            .arg(&repo)
            .env("JJ_CONFIG", empty_config())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "cloning {FIXTURE_URL} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        repo
    })
}

/// A command that runs the server binary in `dir`.
pub fn server_command(dir: &Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_jj-read-only-mcp"));
    command
        .current_dir(dir)
        .env("JJ_CONFIG", empty_config())
        .env("TZ", "UTC");
    command
}

/// A running server, with a client connected to it.
pub struct Server(RunningService<RoleClient, ()>);

impl Server {
    /// Spawns the server in the fixture repo.
    pub async fn spawn() -> Self {
        let transport = TokioChildProcess::new(server_command(repo())).unwrap();
        Self(().serve(transport).await.unwrap())
    }

    /// Calls `tool` and returns its text output and `isError`.
    pub async fn call(&self, tool: &'static str, args: Value) -> (String, bool) {
        let Value::Object(args) = args else {
            panic!("tool args must be an object");
        };
        let result = self
            .0
            .call_tool(CallToolRequestParams::new(tool).with_arguments(args))
            .await
            .unwrap();
        let text: Vec<&str> = result
            .content
            .iter()
            .filter_map(|content| content.as_text())
            .map(|content| content.text.as_str())
            .collect();
        (text.join("\n"), result.is_error.unwrap_or(false))
    }

    /// Calls `tool`, asserts it succeeded, and returns its output.
    pub async fn ok(&self, tool: &'static str, args: Value) -> String {
        let (text, is_error) = self.call(tool, args).await;
        assert!(!is_error, "{tool} failed:\n{text}");
        text
    }

    /// Calls `tool`, asserts it failed, and returns its output.
    pub async fn err(&self, tool: &'static str, args: Value) -> String {
        let (text, is_error) = self.call(tool, args).await;
        assert!(is_error, "{tool} succeeded:\n{text}");
        text
    }

    /// The names of the server's tools.
    pub async fn tool_names(&self) -> Vec<String> {
        let tools = self.0.list_all_tools().await.unwrap();
        tools
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect()
    }
}
