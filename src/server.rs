//! The MCP server: one tool per read-only jj command.

use std::sync::{Arc, Mutex, PoisonError};

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;

use crate::jj::Args;

const INSTRUCTIONS: &str = "\
Read-only access to the jj (Jujutsu) repo in this server's working directory. \
Use it to inspect any revision without checking it out.

Every jj call runs with --ignore-working-copy, so `@` is the working copy as of \
its last snapshot and may be stale. Prefer change IDs, bookmarks or other revsets to `@`.

Arguments:
- `revset`: a jj revset (jj_help topic `revsets`).
- `revision`: a revset that must match a single change.
- `paths`: fileset expressions (jj_help topic `filesets`). Plain relative paths work.
- `template`: a jj template (jj_help topic `templates`).

Call jj_set_default_revision once and every omitted `revset`/`revision` param uses it.

Each result starts with the exact command run (`$ jj ...`), then jj's stdout and \
stderr. If jj fails, the result is an error and ends with `exit code: N`.

Suggested review flow:
1. jj_set_default_revision with the change under review.
2. jj_show for its description and changed files.
3. jj_diff with `paths` to read the diff a file or directory at a time.
4. jj_file_show to read whole files at that revision for context.";

#[derive(Deserialize, schemars::JsonSchema)]
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
pub struct ShowArgs {
    /// Revision to show. Defaults to the default revision.
    revset: Option<String>,
    /// Template for the commit header.
    template: Option<String>,
}

/// Output format shared by the diff tools. Git format unless one is set.
#[derive(Deserialize, schemars::JsonSchema)]
pub struct DiffFormat {
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
pub struct DiffArgs {
    /// Revset whose changes to show. Several revisions are shown as one
    /// combined diff. Defaults to the default revision.
    revset: Option<String>,
    /// Only show changes to these filesets.
    #[serde(default)]
    paths: Vec<String>,
    #[serde(flatten)]
    format: DiffFormat,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct RangeArgs {
    /// Revision to diff from. Required: the default revision is not used.
    from: String,
    /// Revision to diff to. Required: the default revision is not used.
    to: String,
    /// Only show changes to these filesets.
    #[serde(default)]
    paths: Vec<String>,
    #[serde(flatten)]
    format: DiffFormat,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct EvologArgs {
    /// Revision whose history to show. Defaults to the default revision.
    revision: Option<String>,
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
pub struct FileShowArgs {
    /// Revision to read the files at. Defaults to the default revision.
    revision: Option<String>,
    /// Filesets to print.
    paths: Vec<String>,
    /// Template for each file.
    template: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct FileListArgs {
    /// Revision to list the files at. Defaults to the default revision.
    revision: Option<String>,
    /// Only list files matching these filesets.
    #[serde(default)]
    paths: Vec<String>,
    /// Template for each file.
    template: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct FileSearchArgs {
    /// Revision to search at. Defaults to the default revision.
    revision: Option<String>,
    /// String pattern matched against each line, as `kind:pattern` (see
    /// "String patterns" in `jj_help` topic `revsets`). The kind defaults to
    /// `regex`. A `glob:` pattern must match the whole line.
    pattern: String,
    /// Print only the names of matching files, not the matching lines.
    #[serde(default)]
    name_only: bool,
    /// Only search files matching these filesets.
    #[serde(default)]
    paths: Vec<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct FileAnnotateArgs {
    /// Revision to annotate the file at. Defaults to the default revision.
    revision: Option<String>,
    /// Path of one file (a literal path, not a fileset).
    path: String,
    /// Template for each line.
    template: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
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
pub struct HelpArgs {
    /// One of `bookmarks`, `config`, `filesets`, `glossary`, `revsets`,
    /// `templates`, `tutorial`.
    topic: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SetDefaultArgs {
    /// Revision to use when a tool's `revset`/`revision` is omitted. Null or
    /// empty clears the default.
    revision: Option<String>,
}

type ToolResult = Result<CallToolResult, CallToolResult>;

#[derive(Clone)]
pub struct JjServer {
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

    fn default_revision(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.default_revision
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Returns `given`, or the default revision if `given` is omitted or
    /// empty. Errors if neither is set.
    fn resolve(&self, given: Option<String>, param: &str) -> Result<String, CallToolResult> {
        given
            .filter(|given| !given.is_empty())
            .or_else(|| self.default_revision().clone())
            .ok_or_else(|| {
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
        is not snapshotted."
    )]
    async fn jj_log(&self, Parameters(args): Parameters<LogArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset")?;
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
        files (`jj show --stat`). Use jj_diff for the patch."
    )]
    async fn jj_show(&self, Parameters(args): Parameters<ShowArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset")?;
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
        output small."
    )]
    async fn jj_diff(&self, Parameters(args): Parameters<DiffArgs>) -> ToolResult {
        let revset = self.resolve(args.revset, "revset")?;
        let DiffFormat {
            stat,
            summary,
            name_only,
            context,
        } = args.format;
        Ok(Args::new(&["diff"])
            .value("--revisions", revset)
            .diff_format(stat, summary, name_only, context)
            .positional(args.paths)
            .run()
            .await)
    }

    #[tool(
        description = "Show the difference between the trees of two revisions (`jj diff \
        --from --to`), in git format unless `stat`, `summary` or `name_only` is set. \
        `from` and `to` are required: the default revision is not used."
    )]
    async fn jj_diff_range(&self, Parameters(args): Parameters<RangeArgs>) -> CallToolResult {
        range_diff("diff", args).await
    }

    #[tool(
        description = "Compare the changes made by two revisions (`jj interdiff --from \
        --to`): the difference between their patches, not their trees. Useful to see what \
        changed between two versions of a change, e.g. across a rebase. In git format \
        unless `stat`, `summary` or `name_only` is set. `from` and `to` are required: the \
        default revision is not used."
    )]
    async fn jj_interdiff(&self, Parameters(args): Parameters<RangeArgs>) -> CallToolResult {
        range_diff("interdiff", args).await
    }

    #[tool(
        description = "Show how a change evolved (`jj evolog`): its earlier versions from \
        rewrites such as amends and rebases."
    )]
    async fn jj_evolog(&self, Parameters(args): Parameters<EvologArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision")?;
        Ok(Args::new(&["evolog"])
            .value("--revisions", revision)
            .opt("--limit", args.limit)
            .flag(args.no_graph, "--no-graph")
            .flag(args.patch, "--patch")
            .flag(args.patch, "--git")
            .opt("--template", args.template)
            .run()
            .await)
    }

    #[tool(description = "Print the contents of files at a revision (`jj file show`).")]
    async fn jj_file_show(&self, Parameters(args): Parameters<FileShowArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision")?;
        Ok(Args::new(&["file", "show"])
            .value("--revision", revision)
            .opt("--template", args.template)
            .positional(args.paths)
            .run()
            .await)
    }

    #[tool(description = "List the files at a revision (`jj file list`).")]
    async fn jj_file_list(&self, Parameters(args): Parameters<FileListArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision")?;
        Ok(Args::new(&["file", "list"])
            .value("--revision", revision)
            .opt("--template", args.template)
            .positional(args.paths)
            .run()
            .await)
    }

    #[tool(
        description = "Search file contents at a revision (`jj file search`). Prints \
        matching lines with their line numbers, or only file names if `name_only` is set."
    )]
    async fn jj_file_search(&self, Parameters(args): Parameters<FileSearchArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision")?;
        Ok(Args::new(&["file", "search"])
            .value("--revision", revision)
            .value("--pattern", args.pattern)
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
        revision (`jj file annotate`, like `git blame`)."
    )]
    async fn jj_file_annotate(&self, Parameters(args): Parameters<FileAnnotateArgs>) -> ToolResult {
        let revision = self.resolve(args.revision, "revision")?;
        Ok(Args::new(&["file", "annotate"])
            .value("--revision", revision)
            .opt("--template", args.template)
            .positional(vec![args.path])
            .run()
            .await)
    }

    #[tool(
        description = "List bookmarks and their targets (`jj bookmark list`). The \
        default revision is not used here."
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
        bookmarks, config, filesets, glossary, revsets, templates, tutorial."
    )]
    async fn jj_help(&self, Parameters(args): Parameters<HelpArgs>) -> CallToolResult {
        Args::new(&["help"])
            .value("--keyword", args.topic)
            .run()
            .await
    }

    #[tool(
        description = "Show the working copy's status (`jj status`). The working copy \
        is not snapshotted, so this is its state at the last snapshot and may be stale."
    )]
    async fn jj_status(&self) -> CallToolResult {
        Args::new(&["status"]).run().await
    }

    #[tool(
        description = "Set the revision used when a tool's `revset`/`revision` param is \
        omitted. It is kept in memory for this session only. Runs `jj log` on the revision \
        and returns its output. The default is only changed if that succeeds. Pass null or \
        an empty string to clear the default."
    )]
    async fn jj_set_default_revision(
        &self,
        Parameters(args): Parameters<SetDefaultArgs>,
    ) -> CallToolResult {
        let Some(revision) = args.revision.filter(|revision| !revision.is_empty()) else {
            *self.default_revision() = None;
            return CallToolResult::success(vec![ContentBlock::text("Default revision cleared.")]);
        };
        let result = Args::new(&["log"])
            .arg("--no-graph")
            .value("--revision", &revision)
            .run()
            .await;
        if result.is_error != Some(true) {
            *self.default_revision() = Some(revision);
        }
        result
    }
}

async fn range_diff(subcommand: &str, args: RangeArgs) -> CallToolResult {
    let DiffFormat {
        stat,
        summary,
        name_only,
        context,
    } = args.format;
    Args::new(&[subcommand])
        .value("--from", args.from)
        .value("--to", args.to)
        .diff_format(stat, summary, name_only, context)
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
}
