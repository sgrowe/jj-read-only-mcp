use std::fmt;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::{Deserialize, Serialize};

use super::ToolResult;
use crate::jj::Args;
use crate::server::JjServer;

/// How `jj_file_search` matches its pattern (jj's string pattern kinds).
#[derive(Clone, Copy, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
enum PatternKind {
    Exact,
    Glob,
    #[default]
    Regex,
    Substring,
}

impl fmt::Display for PatternKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Exact => "exact",
            Self::Glob => "glob",
            Self::Regex => "regex",
            Self::Substring => "substring",
        })
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileSearchArgs {
    /// Revision to search at. Defaults to the default revision.
    revision: Option<String>,
    /// Text to match against each line, used as-is. Set the kind with `kind`,
    /// not a `kind:` prefix.
    pattern: String,
    /// `regex` (the default) matches anywhere in the line, `substring` matches
    /// plain text anywhere in the line, `exact` must equal the whole line, and
    /// `glob` must match the whole line (e.g. `*foo*`).
    #[serde(default)]
    kind: PatternKind,
    /// Match case-insensitively.
    #[serde(default)]
    ignore_case: bool,
    /// Print only the names of matching files, not the matching lines.
    #[serde(default)]
    name_only: bool,
    /// Only search files matching these filesets.
    #[serde(default)]
    paths: Vec<String>,
}

#[tool_router(router = file_search_router, vis = "pub(super)")]
impl JjServer {
    #[tool(
        description = "Search file contents at a revision (`jj file search`). Prints \
        matching lines with their line numbers, or only file names if `name_only` is set.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_search(&self, Parameters(args): Parameters<FileSearchArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision").await?;
        let case = if args.ignore_case { "-i" } else { "" };
        Ok(Args::new(&["file", "search"])
            .value("--revision", revision)
            .value("--pattern", format!("{}{case}:{}", args.kind, args.pattern))
            .arg(if args.name_only {
                "--name-only"
            } else {
                "--line-number"
            })
            .positional(args.paths)
            .run()
            .await)
    }
}
