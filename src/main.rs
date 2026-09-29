mod jj;
mod server;

use std::ffi::OsString;
use std::io::Write;
use std::process::{Command, Stdio, exit};

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `args_os`, not `args`: `args` panics on an arg that isn't UTF-8.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {}
        [arg] if arg == "instructions" => {
            println!("{}", server::INSTRUCTIONS);
            return Ok(());
        }
        _ => {
            eprintln!(
                "usage:\n  \
                 jj-read-only-mcp               run the MCP server on stdio\n  \
                 jj-read-only-mcp instructions  print the server instructions"
            );
            exit(2);
        }
    }
    check_repo();
    server::JjServer::new()
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

/// Exits with jj's error if the current directory is not in a jj repo. The
/// output is captured so `jj root` can't write to the protocol stream.
fn check_repo() {
    let output = Command::new("jj")
        .args(["root", "--ignore-working-copy", "--color=never"])
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            // Ignore write errors: we exit with an error either way.
            let _ = std::io::stderr().write_all(&output.stderr);
            exit(1);
        }
        Err(err) => {
            eprintln!("failed to run jj: {err}");
            exit(1);
        }
    }
}
