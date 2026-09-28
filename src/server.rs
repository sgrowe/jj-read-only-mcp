//! The MCP server: one tool per read-only jj command.

use std::fmt;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::jj::Args;

const INSTRUCTIONS: &str = "\
Read-only access to the jj (Jujutsu) repo in this server's working directory. \
Use it to inspect any revision without checking it out.

Every jj call runs with --ignore-working-copy, so `@` is the working copy as of \
its last snapshot and may be stale. Prefer change IDs, bookmarks or other revsets to `@`.

Arguments:
- `revset`: a jj revset (jj_help topic `revsets`).
- `revision`: a revset that must match a single change.
- `paths`: fileset expressions (jj_help topic `filesets`). Plain paths work, but a path \
with special characters such as `(`, `)`, `~`, `&`, `|` or spaces must be double-quoted \
inside the string, e.g. `\"app/(auth)/page.tsx\"`.
- `template`: a jj template (jj_help topic `templates`).

Call jj_set_default_revision once and every omitted `revset`/`revision` param uses it \
(except in jj_bookmark_list).

Each result starts with the exact command run (`$ jj ...`), then jj's stdout and \
stderr. If jj fails, the result is an error and ends with `exit code: N`.

Suggested review flow:
1. jj_set_default_revision with the change under review.
2. jj_show for its description and changed files.
3. jj_diff with `paths` to read the diff a file or directory at a time.
4. jj_file_show to read whole files at that revision for context.

Tips:
- To review a branch's own changes, use jj_diff_range with \
`from: \"fork_point(trunk() | <branch>)\"` and `to: \"<branch>\"`. \
`from: \"trunk()\"` also shows trunk's newer commits as reverted, and jj_diff \
`trunk()..<branch>` fails if the branch merged trunk.";

/// `jj_file_show`'s default template: a header before each file. It starts
/// with a newline so the header is on its own line even after a file with no
/// final newline.
const FILE_SHOW_TEMPLATE: &str = r#""\n==> " ++ format_path(path) ++ " <==\n""#;

// Every args struct denies unknown fields, so a misnamed param (e.g.
// `revision` where the tool takes `revset`) is an error, not silently
// replaced by the default revision.

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is one jj flag in the tool's schema"
)]
pub struct LogArgs {
    /// Revset to show. Defaults to the default revision.
    revset: Option<String>,
    /// Template for each commit.
    template: Option<String>,
    /// Show at most this many commits.
    limit: Option<u32>,
    /// Show a flat list instead of a graph.
    #[serde(default)]
    no_graph: bool,
    /// Show the oldest commits first.
    #[serde(default)]
    reversed: bool,
    /// Show each commit's patch (git format).
    #[serde(default)]
    patch: bool,
    /// Show each commit's changed files. Can't be combined with `stat`.
    #[serde(default)]
    summary: bool,
    /// Show each commit's diffstat. Can't be combined with `summary`.
    #[serde(default)]
    stat: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShowArgs {
    /// Revset to show. Defaults to the default revision.
    revset: Option<String>,
    /// Template for the commit header.
    template: Option<String>,
}

// The format fields of `DiffArgs` and `RangeArgs` are repeated rather than
// shared with `#[serde(flatten)]`, which doesn't work with
// `deny_unknown_fields`.

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiffArgs {
    /// Revset whose changes to show. Several revisions are shown as one
    /// combined diff. Defaults to the default revision.
    revset: Option<String>,
    /// Only show changes to these filesets.
    #[serde(default)]
    paths: Vec<String>,
    /// Show a diffstat instead of the patch. Only one of `stat`, `summary`
    /// and `name_only` can be set.
    #[serde(default)]
    stat: bool,
    /// Show changed files with their status (A/M/D) instead of the patch.
    /// Only one of `stat`, `summary` and `name_only` can be set.
    #[serde(default)]
    summary: bool,
    /// Show only the changed file names. Only one of `stat`, `summary` and
    /// `name_only` can be set.
    #[serde(default)]
    name_only: bool,
    /// Lines of context around each change in the patch.
    context: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RangeArgs {
    /// Revision to diff from. Required: the default revision is not used.
    from: String,
    /// Revision to diff to. Required: the default revision is not used.
    to: String,
    /// Only show changes to these filesets.
    #[serde(default)]
    paths: Vec<String>,
    /// Show a diffstat instead of the patch. Only one of `stat`, `summary`
    /// and `name_only` can be set.
    #[serde(default)]
    stat: bool,
    /// Show changed files with their status (A/M/D) instead of the patch.
    /// Only one of `stat`, `summary` and `name_only` can be set.
    #[serde(default)]
    summary: bool,
    /// Show only the changed file names. Only one of `stat`, `summary` and
    /// `name_only` can be set.
    #[serde(default)]
    name_only: bool,
    /// Lines of context around each change in the patch.
    context: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvologArgs {
    /// Revset whose history to show. Defaults to the default revision.
    revset: Option<String>,
    /// Show at most this many versions.
    limit: Option<u32>,
    /// Show a flat list instead of a graph.
    #[serde(default)]
    no_graph: bool,
    /// Show the patch between each version and the one before (git format).
    #[serde(default)]
    patch: bool,
    /// Template for each version. Keywords are on `commit`, e.g.
    /// `commit.commit_id()`, not `commit_id`.
    template: Option<String>,
}

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

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileListArgs {
    /// Revision to list the files at. Defaults to the default revision.
    revision: Option<String>,
    /// Only list files matching these filesets.
    #[serde(default)]
    paths: Vec<String>,
    /// Template for each file.
    template: Option<String>,
}

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

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileAnnotateArgs {
    /// Revision to annotate the file at. Defaults to the default revision.
    revision: Option<String>,
    /// Path of one file (a literal path, not a fileset).
    path: String,
    /// Template for each line.
    template: Option<String>,
}

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

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HelpArgs {
    /// One of `bookmarks`, `config`, `filesets`, `glossary`, `revsets`,
    /// `templates`, `tutorial`.
    topic: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDefaultArgs {
    /// Revision to use when a tool's `revset`/`revision` is omitted. Null or
    /// empty clears the default.
    revision: Option<String>,
}

type ToolResult = Result<CallToolResult, CallToolResult>;

#[derive(Clone)]
pub struct JjServer {
    /// An async mutex, so `jj_set_default_revision` can hold it while it
    /// checks the new value. Calls then see the default in the order they
    /// were made, not the order their `jj log` checks finished.
    default_revision: Arc<Mutex<Option<String>>>,
    tool_router: ToolRouter<Self>,
}

impl JjServer {
    pub fn new() -> Self {
        Self {
            default_revision: Arc::default(),
            tool_router: Self::tool_router(),
        }
    }

    /// Returns `given`, or the default revision if `given` is omitted or
    /// empty. Errors if neither is set.
    async fn resolve(&self, given: Option<String>, param: &str) -> Result<String, CallToolResult> {
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

#[tool_router]
impl JjServer {
    #[tool(
        description = "Show the commits in a revset (`jj log`), optionally with each \
        commit's patch, changed files or diffstat. `@` may be stale: the working copy \
        is not snapshotted.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_log(&self, Parameters(args): Parameters<LogArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset").await?;
        Ok(Args::new(&["log"])
            .value("--revision", revset)
            .opt("--template", args.template)
            .opt("--limit", args.limit)
            .flag(args.no_graph, "--no-graph")
            .flag(args.reversed, "--reversed")
            .flag(args.patch, "--patch")
            .flag(args.patch, "--git")
            .flag(args.summary, "--summary")
            .flag(args.stat, "--stat")
            .run()
            .await)
    }

    #[tool(
        description = "Show a revision's description and a diffstat of its changed \
        files (`jj show --stat`). Use jj_diff for the patch.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_show(&self, Parameters(args): Parameters<ShowArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset").await?;
        Ok(Args::new(&["show"])
            .arg("--stat")
            .opt("--template", args.template)
            .positional(vec![revset])
            .run()
            .await)
    }

    #[tool(
        description = "Show the changes made in a revset (`jj diff --revisions`), in git \
        format unless `stat`, `summary` or `name_only` is set. Use `paths` to keep the \
        output small.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_diff(&self, Parameters(args): Parameters<DiffArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset").await?;
        Ok(Args::new(&["diff"])
            .value("--revisions", revset)
            .diff_format(args.stat, args.summary, args.name_only, args.context)
            .positional(args.paths)
            .run()
            .await)
    }

    #[tool(
        description = "Show the difference between the trees of two revisions (`jj diff \
        --from --to`), in git format unless `stat`, `summary` or `name_only` is set. \
        `from` and `to` are required: the default revision is not used.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_diff_range(&self, Parameters(args): Parameters<RangeArgs>) -> CallToolResult {
        range_diff("diff", args).await
    }

    #[tool(
        description = "Compare the changes made by two revisions (`jj interdiff --from \
        --to`): the difference between their patches, not their trees. Useful to see what \
        changed between two versions of a change, e.g. across a rebase. In git format \
        unless `stat`, `summary` or `name_only` is set. `from` and `to` are required: the \
        default revision is not used.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_interdiff(&self, Parameters(args): Parameters<RangeArgs>) -> CallToolResult {
        range_diff("interdiff", args).await
    }

    #[tool(
        description = "Show how changes evolved (`jj evolog`): their earlier versions from \
        rewrites such as amends and rebases.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_evolog(&self, Parameters(args): Parameters<EvologArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset").await?;
        Ok(Args::new(&["evolog"])
            .value("--revisions", revset)
            .opt("--limit", args.limit)
            .flag(args.no_graph, "--no-graph")
            .flag(args.patch, "--patch")
            .flag(args.patch, "--git")
            .opt("--template", args.template)
            .run()
            .await)
    }

    #[tool(
        description = "Print the contents of files at a revision (`jj file show`). Each \
        file is preceded by a `==> path <==` header, unless `template` is set.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_show(&self, Parameters(args): Parameters<FileShowArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision").await?;
        Ok(Args::new(&["file", "show"])
            .value("--revision", revision)
            .value(
                "--template",
                args.template.as_deref().unwrap_or(FILE_SHOW_TEMPLATE),
            )
            .positional(args.paths)
            .run()
            .await)
    }

    #[tool(
        description = "List the files at a revision (`jj file list`).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_list(&self, Parameters(args): Parameters<FileListArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision").await?;
        Ok(Args::new(&["file", "list"])
            .value("--revision", revision)
            .opt("--template", args.template)
            .positional(args.paths)
            .run()
            .await)
    }

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

    #[tool(
        description = "Show the change that last modified each line of a file at a \
        revision (`jj file annotate`, like `git blame`).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_file_annotate(&self, Parameters(args): Parameters<FileAnnotateArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision").await?;
        Ok(Args::new(&["file", "annotate"])
            .value("--revision", revision)
            .opt("--template", args.template)
            .positional(vec![args.path])
            .run()
            .await)
    }

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

    #[tool(
        description = "Show the working copy's status (`jj status`). The working copy \
        is not snapshotted, so this is its state at the last snapshot and may be stale.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn jj_status(&self) -> CallToolResult {
        Args::new(&["status"]).run().await
    }

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

async fn range_diff(subcommand: &str, args: RangeArgs) -> CallToolResult {
    Args::new(&[subcommand])
        .value("--from", args.from)
        .value("--to", args.to)
        .diff_format(args.stat, args.summary, args.name_only, args.context)
        .positional(args.paths)
        .run()
        .await
}

#[tool_handler(router = self.tool_router)]
#[allow(
    clippy::unused_async_trait_impl,
    reason = "the async fns are generated by `tool_handler`"
)]
impl ServerHandler for JjServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(INSTRUCTIONS)
    }

    /// Replaces the `call_tool` that `tool_handler` would generate, to stop
    /// the tool when the client cancels the call. rmcp only cancels a token,
    /// so without this the tool's jj process runs to the end. Dropping the
    /// tool's future kills it.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let cancelled = context.ct.clone();
        let call = self
            .tool_router
            .call(ToolCallContext::new(self, request, context));
        tokio::select! {
            result = call => result,
            () = cancelled.cancelled() => {
                Ok(CallToolResult::error(vec![ContentBlock::text("Cancelled.")]).into())
            }
        }
    }
}
