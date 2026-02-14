use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// Initialize gpui-component globals and dark theme for tests.
/// Must be called inside `cx.update(|cx| init_test_theme(cx))`.
pub fn init_test_theme(cx: &mut gpui::App) {
    gpui_component::init(cx);
    crate::theme::setup_dark_theme(cx);
}

/// Run a git command in the given directory, panicking if it fails.
fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("failed to execute git");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Create a temp git repo with a single commit (file.txt = "hello").
pub fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@test.com"]);
    run_git(path, &["config", "user.name", "Test"]);

    std::fs::write(path.join("file.txt"), "hello").unwrap();

    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);

    dir
}

/// Create a temp git repo with 2 commits (for diff testing).
/// Commit 1: file.txt = "hello"
/// Commit 2: file.txt = "hello world"
pub fn init_test_repo_with_changes() -> TempDir {
    let dir = init_test_repo();
    let path = dir.path();

    std::fs::write(path.join("file.txt"), "hello world").unwrap();

    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "second commit"]);

    dir
}
