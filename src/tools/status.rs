use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};

use crate::jj::Args;
use crate::server::JjServer;

#[tool_router(router = status_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show the working copy's status (`jj status`). The working copy \
        is not snapshotted, so this is its state at the last snapshot and may be stale.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_status(&self) -> CallToolResult {
        Args::new(&["status"]).run().await
    }
}
