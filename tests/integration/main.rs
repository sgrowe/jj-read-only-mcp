//! Runs each tool against a clone of the fixture repo. Uses `main` (the same
//! commit as tag `v0.1.5`) and `ci_update@origin`, never `@`. Tests that need
//! their own repo make one in a temp dir.

mod bookmark_list;
mod cli;
mod common;
mod default_revision;
mod diff;
mod evolog;
mod file_annotate;
mod file_list;
mod file_search;
mod file_show;
mod help;
mod list_remotes;
mod log;
mod range_diff;
mod server;
mod show;
mod status;
