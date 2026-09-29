use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use crate::jj::Args;
use crate::server::JjServer;

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDefaultArgs {
    /// Revision to use when a tool's `revset`/`revision` is omitted. Null or
    /// empty clears the default.
    revision: Option<String>,
}

impl JjServer {
    /// Returns `given`, or the default revision if `given` is omitted or
    /// empty. Errors if neither is set.
    pub(super) async fn resolve(
        &self,
        given: Option<String>,
        param: &str,
    ) -> Result<String, CallToolResult> {
        if let Some(given) = given.filter(|given| !given.is_empty()) {
            return Ok(given);
        }
        self.default_revision.lock().await.clone().ok_or_else(|| {
            CallToolResult::error(vec![ContentBlock::text(format!(
                "No `{param}` given and no default revision set. Pass `{param}`, \
                 or call jj_set_default_revision first."
            ))])
        })
    }
}

#[tool_router(router = default_revision_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Set the revision used when a tool's `revset`/`revision` param is \
        omitted. It is kept in memory for this session only. Runs `jj log` on the revision \
        and returns its output. The default is only changed if that succeeds. Pass null or \
        an empty string to clear the default.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn jj_set_default_revision(
        &self,
        Parameters(args): Parameters<SetDefaultArgs>,
    ) -> CallToolResult {
        let mut default = self.default_revision.lock().await;
        let Some(revision) = args.revision.filter(|revision| !revision.is_empty()) else {
            *default = None;
            return CallToolResult::success(vec![ContentBlock::text("Default revision cleared.")]);
        };
        let result = Args::new(&["log"])
            .arg("--no-graph")
            .value("--revision", &revision)
            .run()
            .await;
        if result.is_error != Some(true) {
            *default = Some(revision);
        }
        result
    }
}
