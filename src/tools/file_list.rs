use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileListArgs {
    /// Revision to list the files at.
    revision: String,
    /// Only list files matching these filesets.
    #[serde(default)]
    paths: Vec<String>,
    /// Template for each file.
    template: Option<String>,
}

#[tool_router(router = file_list_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "List the files at a revision (`jj file list`).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_list(&self, Parameters(args): Parameters<FileListArgs>) -> CallToolResult {
        Args::new(&["file", "list"])
            .value("--revision", args.revision)
            .opt("--template", args.template)
            .positional(args.paths)
            .run()
            .await
    }
}
