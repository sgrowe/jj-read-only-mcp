use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShowArgs {
    /// Revset to show.
    revset: String,
    /// Template for the commit header.
    template: Option<String>,
}

#[tool_router(router = show_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show a revision's description and a diffstat of its changed \
        files (`jj show --stat`). Use jj_diff for the patch.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_show(&self, Parameters(args): Parameters<ShowArgs>) -> CallToolResult {
        Args::new(&["show"])
            .arg("--stat")
            .opt("--template", args.template)
            .positional(vec![args.revset])
            .run()
            .await
    }
}
