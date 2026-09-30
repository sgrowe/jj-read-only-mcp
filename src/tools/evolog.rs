use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvologArgs {
    /// Revset whose history to show.
    revset: String,
    /// Show at most this many versions.
    limit: Option<u32>,
    /// Show a flat list instead of a graph.
    #[serde(default)]
    no_graph: bool,
    /// Show the patch between each version and the one before (git format).
    #[serde(default)]
    patch: bool,
    /// Template for each version. Keywords are on `commit`, e.g.
    /// `commit.commit_id()`, not `commit_id`.
    template: Option<String>,
}

#[tool_router(router = evolog_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show how changes evolved (`jj evolog`): their earlier versions from \
        rewrites such as amends and rebases.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_evolog(&self, Parameters(args): Parameters<EvologArgs>) -> CallToolResult {
        Args::new(&["evolog"])
            .value("--revisions", args.revset)
            .opt("--limit", args.limit)
            .flag(args.no_graph, "--no-graph")
            .flag(args.patch, "--patch")
            .flag(args.patch, "--git")
            .opt("--template", args.template)
            .run()
            .await
    }
}
