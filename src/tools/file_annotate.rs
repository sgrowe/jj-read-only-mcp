use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileAnnotateArgs {
    /// Revision to annotate the file at.
    revision: String,
    /// Path of one file (a literal path, not a fileset).
    path: String,
    /// Template for each line.
    template: Option<String>,
}

#[tool_router(router = file_annotate_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Show the change that last modified each line of a file at a \
        revision (`jj file annotate`, like `git blame`).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_annotate(
        &self,
        Parameters(args): Parameters<FileAnnotateArgs>,
    ) -> CallToolResult {
        Args::new(&["file", "annotate"])
            .value("--revision", args.revision)
            .opt("--template", args.template)
            .positional(vec![args.path])
            .run()
            .await
    }
}
