use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

// The format fields of `DiffArgs` and `RangeArgs` are repeated rather than
// shared with `#[serde(flatten)]`, which doesn't work with
// `deny_unknown_fields`.

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiffArgs {
    /// Revset whose changes to show. Several revisions are shown as one
    /// combined diff.
    revset: String,
    /// Only show changes to these filesets.
    #[serde(default)]
    paths: Vec<String>,
    /// Show a diffstat instead of the patch. Only one of `stat`, `summary`
    /// and `name_only` can be set.
    #[serde(default)]
    stat: bool,
    /// Show changed files with their status (A/M/D) instead of the patch.
    /// Only one of `stat`, `summary` and `name_only` can be set.
    #[serde(default)]
    summary: bool,
    /// Show only the changed file names. Only one of `stat`, `summary` and
    /// `name_only` can be set.
    #[serde(default)]
    name_only: bool,
    /// Lines of context around each change in the patch.
    context: Option<u32>,
}

#[tool_router(router = diff_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show the changes made in a revset (`jj diff --revisions`), in git \
        format unless `stat`, `summary` or `name_only` is set. Use `paths` to keep the \
        output small.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_diff(&self, Parameters(args): Parameters<DiffArgs>) -> CallToolResult {
        let cmd = Args::new(&["diff"]).value("--revisions", args.revset);
        let cmd = diff_format(cmd, args.stat, args.summary, args.name_only, args.context);
        cmd.positional(args.paths).run().await
    }
}

/// Adds the diff format flags. Diffs are git-formatted unless another
/// format is asked for.
pub(super) fn diff_format(
    args: Args,
    stat: bool,
    summary: bool,
    name_only: bool,
    context: Option<u32>,
) -> Args {
    args.flag(!(stat || summary || name_only), "--git")
        .flag(stat, "--stat")
        .flag(summary, "--summary")
        .flag(name_only, "--name-only")
        .opt("--context", context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_format_defaults_to_git() {
        let args = diff_format(Args::new(&["diff"]), false, false, false, Some(3));
        assert_eq!(
            args.argv(),
            ["diff", "--ignore-working-copy", "--git", "--context=3"]
        );
        let args = diff_format(Args::new(&["diff"]), true, true, false, None);
        assert_eq!(
            args.argv(),
            ["diff", "--ignore-working-copy", "--stat", "--summary"]
        );
    }
}
