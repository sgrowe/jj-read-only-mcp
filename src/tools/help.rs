use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HelpArgs {
    /// One of `bookmarks`, `config`, `filesets`, `glossary`, `revsets`,
    /// `templates`, `tutorial`.
    topic: String,
}

#[tool_router(router = help_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show jj's help on a topic (`jj help --keyword`). Topics: \
        bookmarks, config, filesets, glossary, revsets, templates, tutorial.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_help(&self, Parameters(args): Parameters<HelpArgs>) -> CallToolResult {
        Args::new(&["help"])
            .value("--keyword", args.topic)
            .run()
            .await
    }
}
