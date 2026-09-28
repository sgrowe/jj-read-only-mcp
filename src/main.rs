mod jj;
mod server;

use std::io::Write;
use std::process::{Command, Stdio, exit};

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
