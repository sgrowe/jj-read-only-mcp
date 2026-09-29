use std::fmt::Display;
use std::process::Stdio;

use rmcp::model::{CallToolResult, ContentBlock};
use tokio::process::Command;

use super::output::{command_line, decode, format_output};

/// Added after the subcommand on every call. `--ignore-working-copy` stops jj
/// from snapshotting (or otherwise touching) the working copy.
const GLOBAL_FLAGS: [&str; 1] = ["--ignore-working-copy"];

/// Passed to jj before the subcommand on every call, but not shown in the
/// `$ jj` line. A flag, not `NO_COLOR`, because user config (`ui.color =
/// "always"`) wins over `NO_COLOR`.
const HIDDEN_FLAGS: [&str; 1] = ["--color=never"];

/// A jj argv (without the leading `jj`), built up one flag at a time.
pub struct Args(Vec<String>);

impl Args {
    /// Starts a command, e.g. `Args::new(&["file", "show"])`.
    pub fn new(subcommand: &[&str]) -> Self {
        let argv = subcommand.iter().chain(&GLOBAL_FLAGS);
        Self(argv.map(|arg| (*arg).to_owned()).collect())
    }

    /// Pushes a fixed arg, e.g. `--stat`.
    pub fn arg(mut self, arg: &str) -> Self {
        self.0.push(arg.to_owned());
        self
    }

    /// Pushes `flag` if `on` is true.
    pub fn flag(self, on: bool, flag: &str) -> Self {
        if on { self.arg(flag) } else { self }
    }

    /// Pushes `--flag=value`. The `=` form stops a value like `--version`
    /// being parsed as a flag.
    pub fn value(mut self, flag: &str, value: impl Display) -> Self {
        self.0.push(format!("{flag}={value}"));
        self
    }

    /// Pushes `--flag=value` if `value` is set.
    pub fn opt(self, flag: &str, value: Option<impl Display>) -> Self {
        match value {
            Some(value) => self.value(flag, value),
            None => self,
        }
    }

    /// Pushes `--` and then `values`, if there are any.
    pub fn positional(mut self, values: Vec<String>) -> Self {
        if !values.is_empty() {
            self.0.push("--".to_owned());
            self.0.extend(values);
        }
        self
    }

    /// Runs `jj` and returns its formatted output. `isError` is set if jj
    /// exits non-zero or can't be started. If the returned future is dropped
    /// (e.g. the call is cancelled), jj is killed.
    pub async fn run(self) -> CallToolResult {
        let output = Command::new("jj")
            .args(HIDDEN_FLAGS)
            .args(&self.0)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await;
        let (text, is_error) = match output {
            Ok(output) => format_output(
                &self.0,
                &decode(&output.stdout, "stdout"),
                &decode(&output.stderr, "stderr"),
                output.status.code(),
            ),
            Err(err) => (
                format!("{}\nfailed to run jj: {err}", command_line(&self.0)),
                true,
            ),
        };
        let content = vec![ContentBlock::text(text)];
        if is_error {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        }
    }

    /// The argv built so far (without the leading `jj`).
    #[cfg(test)]
    pub(crate) fn argv(&self) -> &[String] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_builder() {
        let args = Args::new(&["file", "show"])
            .opt("--template", Some("a b"))
            .opt("--limit", None::<u32>)
            .flag(true, "--no-graph")
            .flag(false, "--reversed")
            .positional(vec![])
            .positional(vec!["x".to_owned()]);
        assert_eq!(
            args.argv(),
            [
                "file",
                "show",
                "--ignore-working-copy",
                "--template=a b",
                "--no-graph",
                "--",
                "x",
            ]
        );
    }
}
