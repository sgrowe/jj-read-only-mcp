use std::borrow::Cow;

/// Formats a finished jj call as the `$ ` line, stdout, stderr, and (on
/// failure) an `exit code: N` line. `exit_code` is `None` if jj was killed by
/// a signal. Returns the text and whether the call failed.
pub(super) fn format_output(
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
pub(super) fn decode<'a>(bytes: &'a [u8], stream: &str) -> Cow<'a, str> {
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
pub(super) fn command_line(argv: &[String]) -> String {
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
