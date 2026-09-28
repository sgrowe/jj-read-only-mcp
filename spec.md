A local stdio MCP server (built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk)) giving read-only access to the jj repo in the current working directory, so agents can review code without checking out the revision first.

## Tools

Every jj call gets `--ignore-working-copy --color=never` (so `@` is the last snapshot and may be stale). Value flags are passed as `--flag=value`. Positional args go after `--`.

| Tool | Command |
|---|---|
| `jj_log` | `jj log --revision=$revset [--template=] [--limit=] [--no-graph] [--reversed] [--patch --git] [--summary \| --stat]` |
| `jj_show` | `jj show --stat [--template=] -- $revset` |
| `jj_diff` | `jj diff --revisions=$revset [--git \| --stat \| --summary \| --name-only] [--context=] [-- $paths...]` |
| `jj_diff_range` | `jj diff --from=$from --to=$to [same format flags] [-- $paths...]` |
| `jj_interdiff` | `jj interdiff --from=$from --to=$to [same format flags] [-- $paths...]` |
| `jj_evolog` | `jj evolog --revisions=$revision [--limit=] [--no-graph] [--patch --git] [--template=]` |
| `jj_file_show` | `jj file show --revision=$revision [--template=] -- $paths...` |
| `jj_file_list` | `jj file list --revision=$revision [--template=] [-- $paths...]` |
| `jj_file_search` | `jj file search --revision=$revision --pattern=$string_pattern (--line-number \| --name-only) [-- $paths...]` |
| `jj_file_annotate` | `jj file annotate --revision=$revision [--template=] -- $path` |
| `jj_bookmark_list` | `jj bookmark list [--revision=$revset] [--all-remotes] [--template=] [-- $names...]` |
| `jj_help` | `jj help --keyword=$help_topic` |
| `jj_status` | `jj status` |
| `jj_set_default_revision` | Not a jj command: sets or clears the default revision (see below) |

Diffs are always git-formatted: `--git` is added unless `stat`, `summary` or `name_only` is set. These three are mutually exclusive, and jj errors if you combine them.

### Default revision

`jj_set_default_revision($revision)` stores the string for this server session (in memory, not persisted). It fills any omitted `$revset`/`$revision` param. It never fills `from`/`to`, bookmark filters or help topics. On set, it runs `jj log --no-graph --revision=$revision` and returns the output. The value is stored only if jj succeeds. Passing null/empty clears it. If a tool needs a revision, none is given and no default is set, the tool returns an error without running jj.

## Output

```
$ <exact argv, shell-quoted>
<stdout>
<stderr>
```

- `isError` is set on a non-zero exit, and a final `exit code: N` line is added.
- If stdout and stderr are both empty, output `(no output)`.
- No truncation.

Example:

```
$ jj show --ignore-working-copy --color=never --stat -- zu
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
- `$string_pattern`: see "String patterns" in `jj help -k revsets`.
- `$paths`: fileset expressions (`jj help -k filesets`).
- `$path`: one literal file path (`jj_file_annotate`), not a fileset.
- `$template`: a jj template (`jj help -k templates`). The keywords depend on the command: `jj_evolog` needs `commit.commit_id()`, not `commit_id`.
- `$help_topic`: currently `bookmarks`, `config`, `filesets`, `glossary`, `revsets`, `templates`, `tutorial`.

## Behaviour

- Uses the user's jj config (aliases, templates). Output colour is forced off.
- At startup, runs `jj root`. If cwd is not a jj repo, exits with jj's error.
- No jj version checks and no input validation: jj reports errors.

## Testing

- Integration tests clone `https://github.com/sgrowe/aider-script` (abandoned, never changes) once per run into cargo's temp dir. Each test spawns the server binary with the rmcp client over stdio.
- Tests run with an empty `JJ_CONFIG` and `TZ=UTC`, and use `insta` snapshots. They use stable refs (`main`, which is also tag `v0.1.5`, and the untracked `ci_update@origin`), never `@`.
- CI runs the tests against a matrix of jj versions (currently `0.45.1`), installed from GitHub release binaries. The Rust toolchain is pinned in `rust-toolchain.toml`.
- Lints: clippy `pedantic`, and `unsafe_code = "forbid"`.
