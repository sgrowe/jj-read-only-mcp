use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};

use crate::jj::Args;
use crate::server::JjServer;

#[tool_router(router = list_remotes_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "List the git remotes and their URLs (`jj git remote list`). Use it to \
        find the hosting repo, e.g. the GitHub `owner/repo` for looking up a branch's pull \
        request.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_list_remotes(&self) -> CallToolResult {
        Args::new(&["git", "remote", "list"]).run().await
    }
}
