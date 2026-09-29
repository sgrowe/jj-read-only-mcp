//! The MCP tools: one tool per read-only jj command. Each file holds a tool's
//! args, description and argv, and a router for it.

mod bookmark_list;
mod default_revision;
mod diff;
mod evolog;
mod file_annotate;
mod file_list;
mod file_search;
mod file_show;
mod help;
mod list_remotes;
mod log;
mod range_diff;
mod show;
mod status;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::CallToolResult;

use crate::server::JjServer;

/// The router for every tool.
pub(crate) fn router() -> ToolRouter<JjServer> {
    JjServer::log_router()
        + JjServer::show_router()
        + JjServer::diff_router()
        + JjServer::range_diff_router()
        + JjServer::evolog_router()
        + JjServer::file_show_router()
        + JjServer::file_list_router()
        + JjServer::file_search_router()
        + JjServer::file_annotate_router()
        + JjServer::bookmark_list_router()
        + JjServer::list_remotes_router()
        + JjServer::help_router()
        + JjServer::status_router()
        + JjServer::default_revision_router()
}

// Every args struct denies unknown fields, so a misnamed param (e.g.
// `revision` where the tool takes `revset`) is an error, not silently
// replaced by the default revision.

type ToolResult = Result<CallToolResult, CallToolResult>;
