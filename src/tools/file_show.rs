use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use super::ToolResult;
use crate::jj::Args;
use crate::server::JjServer;

/// `jj_file_show`'s header template, used when more than one path is given:
/// a header before each file. It starts with a newline so the header is on its own line even after a file with no
/// final newline.
const FILE_SHOW_TEMPLATE: &str = r#""\n==> " ++ format_path(path) ++ " <==\n""#;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileShowArgs {
    /// Revision to read the files at. Defaults to the default revision.
    revision: Option<String>,
    /// Filesets to print.
    paths: Vec<String>,
    /// Template for each file's header.
    template: Option<String>,
}

#[tool_router(router = file_show_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Print the contents of files at a revision (`jj file show`). When \
        more than one path is passed each file is preceded by a `==> path <==` header, unless \
        `template` is set.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_show(&self, Parameters(args): Parameters<FileShowArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision").await?;
        let template = args
            .template
            .as_deref()
            .or((args.paths.len() > 1).then_some(FILE_SHOW_TEMPLATE));
        Ok(Args::new(&["file", "show"])
            .value("--revision", revision)
            .opt("--template", template)
            .positional(args.paths)
            .run()
            .await)
    }
}
