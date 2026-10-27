# `jj` read-only MCP

Give an agent read-only access to any git branch without checking it out locally (for speed and security).

`jj` VCS has some neat commands for searching and viewing files at any arbitrary commit:

```bash
# Search all files in the repo as it exists at the given branch/commit
jj file search --pattern "regex_pattern" --revision branch@origin

# View the full contents of `path/file.ts` as it exists at the given branch/commit
jj file show "path/file.ts" --revision commit123

# See the commit message and list of files added, modified, deleted
jj show commit123 --summary

# `git blame` equivalent
jj file annotate --revision commit123 main.rs

# and many more...
```

## Why

I set up a local agent that runs continuously reviewing PRs, and restricting it to only viewing the repo through this read-only MCP (+ github comments via MCP) has some nice properties:

- it's secure, so I can run it continuously without any worries
- it's fast and token efficient as the model only has access to the exact tools it needs for code review. For example I often find review agents get tempted to run tests locally unless they're explicitly told not to which just wastes time when CI does this anyway.
- minimal bookkeeping needed as we don't need to checkout branches or create worktrees locally, just `jj git fetch` and you have the latest changes ready to go
- in theory this setup would also enable multiple reviews to happen in parallel but I don't really need that atm

I run this using a `pi` agent with [codemode](https://pi.dev/docs/latest/codemode) enabled (and built in tools disabled), which allows the model to chain MCP calls together and filter their output in a JS sandbox before it gets back the result.

## Tools list

**Commits and history:**

- `jj log` - List commits
- `jj show` - Show a commit’s description and changed files
- `jj evolog` - Show earlier versions of a [change](https://docs.jj-vcs.dev/latest/tutorial/#changes)
- `jj bookmark list` - List bookmarks (branches)

**Diffs:**

- `jj diff` - Diff between two commits (or a set of commits)
- `jj interdiff` - Get the diff between two diffs 🤯 [docs](https://docs.jj-vcs.dev/latest/cli-reference/#jj-interdiff)

**Files at a revision:**

- `jj file show` - Show file contents
- `jj file list` - List files
- `jj file search` - Search file contents
- `jj file annotate` - Show who changed each line (`git blame`)

**Other:**

- `jj git remote list` - List git remotes
- `jj status` - Show the working copy's status
- `jj help --keyword` - Show jj's help on a topic

## Usage

```sh
## Install
cargo install --git https://github.com/sgrowe/jj-read-only-mcp

## Run the MCP
jj-read-only-mcp

## Print the instructions the server sends to the agent, e.g. to paste into a skill
jj-read-only-mcp instructions
```

Every `jj` tool call runs with `--ignore-working-copy`, so jj never snapshots or modifies the working copy.

The output of each MCP tool call starts with the exact `jj` command that was run (e.g. `$ jj log -r ...`) so that the agent is able to understand what went wrong and correct itself if it gets an error back or some unexpected output.
