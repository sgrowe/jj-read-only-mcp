Read-only access to the jj (Jujutsu) repo in this server's working directory. Use it to inspect any revision without checking it out.

Every jj call runs with --ignore-working-copy, so `@` is the working copy as of its last snapshot and may be stale. Prefer change IDs, bookmarks or other revsets to `@`.

Arguments:
- `revset`: a jj revset (jj_help topic `revsets`).
- `revision`: a revset that must match a single change.
- `paths`: fileset expressions (jj_help topic `filesets`). Plain paths work, but a path with special characters such as `(`, `)`, `~`, `&`, `|` or spaces must be double-quoted inside the string, e.g. `"app/(auth)/page.tsx"`.
- `template`: a jj template (jj_help topic `templates`).

Call jj_set_default_revision once and every omitted `revset`/`revision` param uses it (except in jj_bookmark_list).

Each result starts with the exact command run (`$ jj ...`), then jj's stdout and stderr. If jj fails, the result is an error and ends with `exit code: N`.

Suggested review flow:
1. jj_set_default_revision with the change under review.
2. jj_show for its description and changed files.
3. jj_diff with `paths` to read the diff a file or directory at a time.
4. jj_file_show to read whole files at that revision for context.

Tips:
- To review a branch's own changes, use jj_diff_range with `from: "fork_point(trunk() | <branch>)"` and `to: "<branch>"`. `from: "trunk()"` also shows trunk's newer commits as reverted, and jj_diff `trunk()..<branch>` fails if the branch merged trunk.
