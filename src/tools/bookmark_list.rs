use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BookmarkListArgs {
    /// Only show bookmarks pointing at commits in this revset. The default
    /// revision is not used here.
    revset: Option<String>,
    /// Also show remote bookmarks, including untracked ones and ones in sync
    /// with their local bookmark.
    #[serde(default)]
    all_remotes: bool,
    /// Template for each bookmark.
    template: Option<String>,
    /// Only show bookmarks whose name matches one of these string patterns
    /// (glob by default).
    #[serde(default)]
    names: Vec<String>,
}

#[tool_router(router = bookmark_list_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "List bookmarks and their targets (`jj bookmark list`). The \
        default revision is not used here.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_bookmark_list(
        &self,
        Parameters(args): Parameters<BookmarkListArgs>,
    ) -> CallToolResult {
        Args::new(&["bookmark", "list"])
            .opt("--revision", args.revset)
            .flag(args.all_remotes, "--all-remotes")
            .opt("--template", args.template)
            .positional(args.names)
            .run()
            .await
    }
}
