//! Test harness: the fixture repo, and an MCP client talking to the server
//! binary over stdio.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use rmcp::model::{CallToolRequestParams, Tool};
use rmcp::serde_json::Value;
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};

pub const FIXTURE_URL: &str = "https://github.com/sgrowe/aider-script";

/// Snapshots `$text`, redacting the working-copy IDs in `jj status` output
/// (they are random for each clone).
macro_rules! assert_snap {
    ($text:expr) => {{
        // Evaluated first: `with_settings!` runs its body in a closure, so
        // `$text` can't `.await` inside it.
        let text = $text;
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            filters => vec![(
                r"(Working copy  \(@\) : )[a-z]+ [0-9a-f]+",
                "$1[change-id] [commit-id]",
            )],
        }, {
            insta::assert_snapshot!(text);
        })
    }};
}
pub(crate) use assert_snap;

fn tmp_dir() -> &'static Path {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
}

/// The environment for every jj and server process the tests run.
/// - An empty `JJ_CONFIG`, so the user's config can't change the output.
/// - A fake home, so jj keeps each repo's config (in `~/.config/jj/repos`)
///   there and not in the user's home.
/// - `TZ=UTC`, so timestamps don't depend on the machine.
fn test_env() -> [(&'static str, PathBuf); 4] {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    let home = HOME.get_or_init(|| {
        let home = tmp_dir().join("home");
        if home.exists() {
            std::fs::remove_dir_all(&home).unwrap();
        }
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(tmp_dir().join("empty-jj-config.toml"), "").unwrap();
        home
    });
    [
        ("JJ_CONFIG", tmp_dir().join("empty-jj-config.toml")),
        ("HOME", home.clone()),
        ("XDG_CONFIG_HOME", home.join(".config")),
        ("TZ", "UTC".into()),
    ]
}

/// Runs jj in `dir`, asserts it succeeded, and returns its stdout.
pub fn jj(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("jj")
        .args(args)
        .current_dir(dir)
        .envs(test_env())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "jj {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// The fixture repo, cloned once per test run.
pub fn repo() -> &'static Path {
    static REPO: OnceLock<PathBuf> = OnceLock::new();
    REPO.get_or_init(|| {
        let repo = tmp_dir().join("aider-script");
        if repo.exists() {
            std::fs::remove_dir_all(&repo).unwrap();
        }
        jj(tmp_dir(), &["git", "clone", FIXTURE_URL, "aider-script"]);
        repo
    })
}

/// A command that runs the server binary in `dir`.
pub fn server_command(dir: &Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_jj-read-only-mcp"));
    command.current_dir(dir).envs(test_env());
    command
}

/// A running server, with a client connected to it.
pub struct Server(RunningService<RoleClient, ()>);

impl Server {
    /// Spawns the server in the fixture repo.
    pub async fn spawn() -> Self {
        Self::spawn_in(repo()).await
    }

    /// Spawns the server in `dir`.
    pub async fn spawn_in(dir: &Path) -> Self {
        Self::spawn_command(server_command(dir)).await
    }

    /// Spawns the server with `command`, e.g. a `server_command` with a
    /// changed env.
    pub async fn spawn_command(command: tokio::process::Command) -> Self {
        let transport = TokioChildProcess::new(command).unwrap();
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

    /// The server's tools, as the client receives them.
    pub async fn tools(&self) -> Vec<Tool> {
        self.0.list_all_tools().await.unwrap()
    }

    /// The server's tools, as pretty JSON.
    pub async fn tools_json(&self) -> String {
        rmcp::serde_json::to_string_pretty(&self.tools().await).unwrap()
    }

    /// The server's instructions, from its initialize response.
    pub fn instructions(&self) -> String {
        let info = self.0.peer_info().expect("no initialize response");
        info.instructions
            .clone()
            .expect("server sent no instructions")
    }
}
