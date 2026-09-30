use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use super::diff::diff_format;
use crate::jj::Args;
use crate::server::JjServer;

// See `DiffArgs` for why the format fields are repeated.

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RangeArgs {
    /// Revision to diff from.
    from: String,
    /// Revision to diff to.
    to: String,
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

#[tool_router(router = range_diff_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show the difference between the trees of two revisions (`jj diff \
        --from --to`), in git format unless `stat`, `summary` or `name_only` is set.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_diff_range(&self, Parameters(args): Parameters<RangeArgs>) -> CallToolResult {
        range_diff("diff", args).await
    }

    #[tool(
        description = "Compare the changes made by two revisions (`jj interdiff --from \
        --to`): the difference between their patches, not their trees. Useful to see what \
        changed between two versions of a change, e.g. across a rebase. In git format \
        unless `stat`, `summary` or `name_only` is set.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_interdiff(&self, Parameters(args): Parameters<RangeArgs>) -> CallToolResult {
        range_diff("interdiff", args).await
    }
}

async fn range_diff(subcommand: &str, args: RangeArgs) -> CallToolResult {
    let cmd = Args::new(&[subcommand])
        .value("--from", args.from)
        .value("--to", args.to);
    diff_format(cmd, args.stat, args.summary, args.name_only, args.context)
        .positional(args.paths)
        .run()
        .await
}
