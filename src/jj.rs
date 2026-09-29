//! Builds and runs read-only `jj` commands, and formats their output.

use std::borrow::Cow;
use std::fmt::Display;
use std::process::Stdio;

use rmcp::model::{CallToolResult, ContentBlock};
use tokio::process::Command;

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

    /// Adds the diff format flags. Diffs are git-formatted unless another
    /// format is asked for.
    pub fn diff_format(
        self,
        stat: bool,
        summary: bool,
        name_only: bool,
        context: Option<u32>,
    ) -> Self {
        self.flag(!(stat || summary || name_only), "--git")
            .flag(stat, "--stat")
            .flag(summary, "--summary")
            .flag(name_only, "--name-only")
            .opt("--context", context)
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
}

/// Formats a finished jj call as the `$ ` line, stdout, stderr, and (on
/// failure) an `exit code: N` line. `exit_code` is `None` if jj was killed by
/// a signal. Returns the text and whether the call failed.
pub fn format_output(
    argv: &[String],
    stdout: &str,
    stderr: &str,
    exit_code: Option<i32>,
) -> (String, bool) {
    let mut out = command_line(argv);
    let mut section = |text: &str| {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(text);
    };
    for stream in [stdout, stderr] {
        if !stream.is_empty() {
            section(stream);
        }
    }
    if stdout.is_empty() && stderr.is_empty() {
        section("(no output)");
    }
    let is_error = exit_code != Some(0);
    match exit_code {
        Some(0) => {}
        Some(code) => section(&format!("exit code: {code}")),
        None => section("exit code: none (killed by a signal)"),
    }
    (out, is_error)
}

/// Decodes jj's output as UTF-8. Invalid bytes are replaced, with a note
/// saying so.
fn decode<'a>(bytes: &'a [u8], stream: &str) -> Cow<'a, str> {
    let mut text = String::from_utf8_lossy(bytes);
    if let Cow::Owned(text) = &mut text {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('(');
        text.push_str(stream);
        text.push_str(" was not valid UTF-8: invalid bytes are shown as U+FFFD)");
    }
    text
}

/// The `$ jj …` line, with each arg shell-quoted.
fn command_line(argv: &[String]) -> String {
    let mut line = "$ jj".to_owned();
    for arg in argv {
        line.push(' ');
        line.push_str(&quote(arg));
    }
    line
}

/// Shell-quotes `arg` only if it needs it. Unlike `shlex`, this leaves `=`
/// alone, so flags print as `--revision=main` rather than `'--revision=main'`.
/// A leading `=` is still quoted, because zsh expands `=word`.
fn quote(arg: &str) -> Cow<'_, str> {
    let safe = |c: char| c.is_ascii_alphanumeric() || "_-./=:@,+%".contains(c);
    if !arg.is_empty() && !arg.starts_with('=') && arg.chars().all(safe) {
        Cow::Borrowed(arg)
    } else {
        Cow::Owned(format!("'{}'", arg.replace('\'', r"'\''")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn quote_leaves_safe_args_alone() {
        assert_eq!(quote("--revision=main"), "--revision=main");
        assert_eq!(quote("main@origin"), "main@origin");
        assert_eq!(quote("src/a-b_c.rs"), "src/a-b_c.rs");
    }

    #[test]
    fn quote_wraps_unsafe_args() {
        assert_eq!(quote("trunk()..@"), "'trunk()..@'");
        assert_eq!(quote("a b"), "'a b'");
        assert_eq!(quote("it's"), r"'it'\''s'");
        assert_eq!(quote(""), "''");
        assert_eq!(quote("=eq"), "'=eq'");
    }

    #[test]
    fn decode_notes_invalid_utf8() {
        assert_eq!(decode(b"ok\n", "stdout"), "ok\n");
        assert_eq!(
            decode(b"a\xffb", "stdout"),
            "a\u{fffd}b\n(stdout was not valid UTF-8: invalid bytes are shown as U+FFFD)"
        );
    }

    #[test]
    fn args_builder() {
        let args = Args::new(&["file", "show"])
            .opt("--template", Some("a b"))
            .opt("--limit", None::<u32>)
            .flag(true, "--no-graph")
            .flag(false, "--reversed")
            .positional(vec![])
            .positional(argv(&["x"]));
        assert_eq!(
            args.0,
            argv(&[
                "file",
                "show",
                "--ignore-working-copy",
                "--template=a b",
                "--no-graph",
                "--",
                "x",
            ])
        );
    }

    #[test]
    fn diff_format_defaults_to_git() {
        let args = Args(vec![]).diff_format(false, false, false, Some(3));
        assert_eq!(args.0, argv(&["--git", "--context=3"]));
        let args = Args(vec![]).diff_format(true, true, false, None);
        assert_eq!(args.0, argv(&["--stat", "--summary"]));
    }

    #[test]
    fn output_success() {
        let (text, is_error) = format_output(&argv(&["log", "-r=a b"]), "out\n", "err\n", Some(0));
        assert_eq!(text, "$ jj log '-r=a b'\nout\nerr\n");
        assert!(!is_error);
    }

    #[test]
    fn output_adds_newlines_between_sections() {
        let (text, _) = format_output(&argv(&["status"]), "out", "err", Some(0));
        assert_eq!(text, "$ jj status\nout\nerr");
    }

    #[test]
    fn output_empty() {
        let (text, is_error) = format_output(&argv(&["status"]), "", "", Some(0));
        assert_eq!(text, "$ jj status\n(no output)");
        assert!(!is_error);
    }

    #[test]
    fn output_failure() {
        let (text, is_error) = format_output(&argv(&["log"]), "", "Error: bad\n", Some(1));
        assert_eq!(text, "$ jj log\nError: bad\nexit code: 1");
        assert!(is_error);

        let (text, is_error) = format_output(&argv(&["log"]), "", "", None);
        assert_eq!(
            text,
            "$ jj log\n(no output)\nexit code: none (killed by a signal)"
        );
        assert!(is_error);
    }
}
