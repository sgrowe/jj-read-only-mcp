# AGENTS.md

A local stdio MCP server that gives agents read-only access to the jj repo in its working directory.

[spec.md](spec.md) is the source of truth for behaviour: the command line, each tool's args and argv, and the output. Keep it in sync with the code.

## VCS

This repo uses jj, colocated with git. Use `jj` commands, not `git`.

## Layout

```
src/
  main.rs              command line: runs the server, or prints the instructions
  server.rs            `JjServer`: setup, instructions, and cancelling tool calls
  instructions.md      the instructions text the server sends
  jj/
    mod.rs             re-exports `Args`
    args.rs            `Args`: builds a jj argv and runs it
    output.rs          formats jj's output as the tool result
  tools/
    mod.rs             `router()`: adds up every tool's router
    <tool>.rs          one tool per file (e.g. `log.rs` is `jj_log`): its args, description and argv
    range_diff.rs      `jj_diff_range` and `jj_interdiff`, which share `RangeArgs`
tests/integration/
  main.rs              the test crate root
  common.rs            test harness: the fixture repo, an MCP client, `assert_snap!`
  cli.rs               command-line tests
  server.rs            tests of the server as a whole, or of many tools at once
  <tool>.rs            the tests for `src/tools/<tool>.rs`
  snapshots/           insta snapshots, named `<test fn>.snap`
```

## Checks

CI runs these:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`. It needs `jj` on `PATH`, and network access to clone the fixture repo.

Review snapshot changes with `cargo insta review`.
