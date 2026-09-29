A local stdio MCP server (built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk)) giving read-only access to the jj repo in the current working directory, so agents can review code without checking out the revision first.

## Command line

- `jj-read-only-mcp`: run the MCP server on stdio, in the current directory's jj repo.
- `jj-read-only-mcp instructions`: print the server instructions (the text sent in the initialize response) and exit. Doesn't need a jj repo. Useful to copy into a CLAUDE.md or skill.
- Any other args: print usage to stderr and exit with code 2.

## Tools

Every jj call gets `--ignore-working-copy` (so `@` is the last snapshot and may be stale) and `--color=never`. `--color=never` is passed before the subcommand and left out of the echoed `$ jj` line. It is a flag, not `NO_COLOR`, because user config (`ui.color = "always"`) wins over `NO_COLOR`. Value flags are passed as `--flag=value`. Positional args go after `--`.

| Tool | Command |
|---|---|
| `jj_log` | `jj log --revision=$revset [--template=] [--limit=] [--no-graph] [--reversed] [--patch --git] [--summary \| --stat]` |
| `jj_show` | `jj show --stat [--template=] -- $revset` |
| `jj_diff` | `jj diff --revisions=$revset [--git \| --stat \| --summary \| --name-only] [--context=] [-- $paths...]` |
| `jj_diff_range` | `jj diff --from=$from --to=$to [same format flags] [-- $paths...]` |
| `jj_interdiff` | `jj interdiff --from=$from --to=$to [same format flags] [-- $paths...]` |
| `jj_evolog` | `jj evolog --revisions=$revset [--limit=] [--no-graph] [--patch --git] [--template=]` |
| `jj_file_show` | `jj file show --revision=$revision [--template=($template, or the header template if 2+ paths)] -- $paths...` |
| `jj_file_list` | `jj file list --revision=$revision [--template=] [-- $paths...]` |
| `jj_file_search` | `jj file search --revision=$revision --pattern=$kind[-i]:$pattern (--line-number \| --name-only) [-- $paths...]` |
| `jj_file_annotate` | `jj file annotate --revision=$revision [--template=] -- $path` |
| `jj_bookmark_list` | `jj bookmark list [--revision=$revset] [--all-remotes] [--template=] [-- $names...]` |
| `jj_help` | `jj help --keyword=$help_topic` |
| `jj_status` | `jj status` |
| `jj_set_default_revision` | Not a jj command: sets or clears the default revision (see below) |

Diffs are always git-formatted: `--git` is added unless `stat`, `summary` or `name_only` is set. These three are mutually exclusive, and jj errors if you combine them.

Unknown params are rejected (the schemas set `additionalProperties: false`), so a misnamed param such as `revision` on a tool that takes `revset` is an error rather than silently falling back to the default revision.

Every tool is annotated `readOnlyHint: true, openWorldHint: false`, except `jj_set_default_revision`, which is `readOnlyHint: false, destructiveHint: false, idempotentHint: true`.

If the client cancels a call, the jj process is killed.

### Default revision

`jj_set_default_revision($revision)` stores the string for this server session (in memory, not persisted). It fills any omitted `$revset`/`$revision` param. It never fills `from`/`to`, bookmark filters or help topics. On set, it runs `jj log --no-graph --revision=$revision` and returns the output. The value is stored only if jj succeeds. Passing null/empty clears it. If a tool needs a revision, none is given and no default is set, the tool returns an error without running jj. Setting takes a lock that is held until its `jj log` check finishes, so calls see the default in the order they were made.

## Output

```
$ <argv, shell-quoted, without the hidden --color=never>
<stdout>
<stderr>
```

- `isError` is set on a non-zero exit, and a final `exit code: N` line is added.
- If stdout and stderr are both empty, output `(no output)`.
- No truncation.
- Output that isn't valid UTF-8 has the invalid bytes replaced with U+FFFD, and a note saying so.

Example:

```
$ jj show --ignore-working-copy --stat -- zu
Commit ID: 24ca6c465e14cf8a68871f49f1baf4ae8040393c
Change ID: zuxuusmnquwysxsnlxlyzuvmtwskzyru
Author   : Sam Rowe <sgrowe@live.co.uk> (2 days ago)
Committer: Sam Rowe <sgrowe@live.co.uk> (46 minutes ago)

    Add v3 permissions

docs/scope-report.md   |  1 +
app/permissions.ts     | 45 +++++++++++++++++++++++++++++++++++----------
2 files changed, 36 insertions(+), 10 deletions(-)
```

## Argument conventions

- `$revset`: a jj revset (`jj help -k revsets`).
- `$revision`: a revset matching a single change. jj errors if it matches several.
- `$kind`: one of `exact`, `glob`, `regex`, `substring` (default `regex`), plus `-i` if `ignore_case` is set. See "String patterns" in `jj help -k revsets`.
- `$pattern`: used verbatim, so a colon in it is never read as a pattern kind.
- `$paths`: fileset expressions (`jj help -k filesets`). Plain paths work, but a path with special characters such as `(`, `)`, `~`, `&`, `|` or spaces must be double-quoted inside the string, e.g. `"app/(auth)/page.tsx"`. Unquoted, `a~b` means "`a` minus `b`".
- `$path`: one literal file path (`jj_file_annotate`), not a fileset.
- `$template`: a jj template (`jj help -k templates`). The keywords depend on the command: `jj_evolog` needs `commit.commit_id()`, not `commit_id`.
- `$help_topic`: currently `bookmarks`, `config`, `filesets`, `glossary`, `revsets`, `templates`, `tutorial`.

## Behaviour

- Uses the user's jj config (aliases, templates). Output colour is forced off.
- `jj_file_show` uses its header template only when given more than one path and no `template`. It prints `==> path <==` before each file, starting with a newline so the header is on its own line after a file with no final newline. With one path (even a directory or glob matching several files), no `--template` is passed, so the user's `templates.file_show` applies.
- At startup, runs `jj root`. If cwd is not a jj repo, exits with jj's error.
- No jj version checks and no input validation beyond the schema: jj reports errors.

### Known limits of the read-only guarantee

`--ignore-working-copy` stops jj touching the working copy, but jj can still write some repo metadata when it loads the repo:

- **Divergent operations.** If the op log has more than one head (e.g. the user ran two jj commands at the same time, or used `jj --at-op=<old op>` to make a change), any tool call merges them, printing "Concurrent modification detected, resolving automatically." This writes a new "reconcile divergent operations" operation. Adding `--at-op=@` to every call would stop this (jj then errors instead), but we don't, because the user's next jj command would do the same merge anyway.
- **Repo config.** jj keeps each repo's config outside the repo, in its config dir (e.g. `~/.config/jj/repos/`), and may create or update that entry (e.g. for a newly cloned or copied repo).

## Testing

- Integration tests clone `https://github.com/sgrowe/aider-script` (abandoned, never changes) once per run into cargo's temp dir. Each test spawns the server binary with the rmcp client over stdio.
- Tests run with an empty `JJ_CONFIG`, `TZ=UTC` and a fake `HOME` (so jj's per-repo config goes in cargo's temp dir, not the user's home), and use `insta` snapshots. They use stable refs (`main`, which is also tag `v0.1.5`, and the untracked `ci_update@origin`), never `@`.
- `token_counts` snapshots the approximate token cost of the instructions and each tool's `{name, description, inputSchema}` (tiktoken `o200k_base`, annotations excluded). Use it to compare changes, not as exact Claude counts.
- `does_not_write_to_the_repo` runs every tool against a colocated repo with an unsnapshotted working-copy change, and checks the op log is unchanged.
- CI runs the tests against a matrix of jj versions (currently `0.45.1`), installed from GitHub release binaries. The Rust toolchain is pinned in `rust-toolchain.toml`.
- Lints: clippy `pedantic`, and `unsafe_code = "forbid"`.
