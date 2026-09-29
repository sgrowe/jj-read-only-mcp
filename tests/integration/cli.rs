use crate::common::{Server, server_command};

#[tokio::test]
async fn exits_outside_a_repo() {
    // Not `CARGO_TARGET_TMPDIR`: that is inside this project's jj repo.
    let dir = tempfile::tempdir().unwrap();
    let output = server_command(dir.path()).output().await.unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("There is no jj repo"), "{stderr}");
}

/// `instructions` prints the same text the server sends, and doesn't need a
/// repo.
#[tokio::test]
async fn instructions_command() {
    let dir = tempfile::tempdir().unwrap();
    let output = server_command(dir.path())
        .arg("instructions")
        .output()
        .await
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let instructions = Server::spawn().await.instructions();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        instructions + "\n"
    );
}

#[tokio::test]
async fn unknown_cli_args_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let output = server_command(dir.path())
        .arg("nope")
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage"), "{stderr}");
}
