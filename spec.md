A local MCP server (built using [rmcp](https://github.com/modelcontextprotocol/rust-sdk)) which provides read only access to the jj repo in the current working directory. This allows agents to perform code reviews of a jj repo without having to check out the given revision first.

### jj commands

There should be an MCP tool mapping to each of these commands:

- `jj log --revision $revset [-T $template] [--limit $limit] [--no-graph] [--patch --git] [--summary] [--stat]`
- `jj show $revset --stat`
- `jj file search --pattern $string_pattern --revision $revision --line-number`
- `jj file show --revision $revision [--template $template]`
- `jj file list --revision $revision [--template $template]`
- `jj file annotate $file --revision $revision [-T $template]`
- `jj diff -r $revset --git [...$paths]`
- `jj help -k $help_topic`
- `jj status`

Every command has `--ignore-working-copy` appended to avoid mutating the local jj state. This is so that reviewer agents can be run against this MCP in the background without any fear they might race or interfere with work and changes going on in the working copy.

Each tool call output has the jj command executed as the first line (prefixed with `$ `), then the verbatim jj output, so that if there's an error the agent can see exactly what command was executed and can debug errors more easily.

### Example tool call response

```
$ jj show zu --stat --ignore-working-copy
Commit ID: 24ca6c465e14cf8a68871f49f1baf4ae8040393c
Change ID: zuxuusmnquwysxsnlxlyzuvmtwskzyru
Author   : Sam Rowe <sgrowe@live.co.uk> (2 days ago)
Committer: Sam Rowe <sgrowe@live.co.uk> (46 minutes ago)

    Add v3 permissions

docs/scope-report.md   |  1 +
app/permissions.ts     | 45 +++++++++++++++++++++++++++++++++++----------
2 files changed, 36 insertions(+), 10 deletions(-)
```

### Arguments conventions

- `$revset` - a jj revset - format is given by `jj help -k revsets`
- `$revision` - a single jj revision. This is a revset which only matches a single change, the underlying `jj` command errors if it matches multiple.
- `$string_pattern` a jj string pattern in the format given by the "## String patterns" section in `jj help -k revsets`
- `$help_topic` - the currently available help topics are `bookmarks`, `config`, `filesets`, `glossary`, `revsets`, `templates`, `tutorial`.

### Additional non jj command tools

- set default revset ($revision is required in the above commands unless this is set, not persisted between sessions)

### Other

- Don't worry about running checks to see what jj version is installed/used.
- Don't worry about validating tool inputs like revsets etc, keep things simple and let jj do that.
- Diffs are always be `--git` formatted.
- Integration tests clone down `https://github.com/sgrowe/aider-script` into a temp directory and run the MCP server against that (that repo is abandoned and won't ever change).
