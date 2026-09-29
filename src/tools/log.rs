use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use super::ToolResult;
use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is one jj flag in the tool's schema"
)]
pub struct LogArgs {
    /// Revset to show. Defaults to the default revision.
    revset: Option<String>,
    /// Template for each commit.
    template: Option<String>,
    /// Show at most this many commits.
    limit: Option<u32>,
    /// Show a flat list instead of a graph.
    #[serde(default)]
    no_graph: bool,
    /// Show the oldest commits first.
    #[serde(default)]
    reversed: bool,
    /// Show each commit's patch (git format).
    #[serde(default)]
    patch: bool,
    /// Show each commit's changed files. Can't be combined with `stat`.
    #[serde(default)]
    summary: bool,
    /// Show each commit's diffstat. Can't be combined with `summary`.
    #[serde(default)]
    stat: bool,
}

#[tool_router(router = log_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show the commits in a revset (`jj log`), optionally with each \
        commit's patch, changed files or diffstat. `@` may be stale: the working copy \
        is not snapshotted.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_log(&self, Parameters(args): Parameters<LogArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset").await?;
        Ok(Args::new(&["log"])
            .value("--revision", revset)
            .opt("--template", args.template)
            .opt("--limit", args.limit)
            .flag(args.no_graph, "--no-graph")
            .flag(args.reversed, "--reversed")
            .flag(args.patch, "--patch")
            .flag(args.patch, "--git")
            .flag(args.summary, "--summary")
            .flag(args.stat, "--stat")
            .run()
            .await)
    }
}
