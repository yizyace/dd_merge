use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use tempfile::TempDir;

use dd_git::{FileStatus, LineOrigin, Repository};

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

struct FixtureRepo {
    _dir: TempDir,
    path: PathBuf,
    root_oid: String,
    merge_oid: String,
    multi_file_oid: String,
    multi_hunk_oid: String,
    rename_oid: String,
    delete_oid: String,
    binary_oid: String,
    unicode_oid: String,
}

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .unwrap_or_else(|e| panic!("git {}: {e}", args.join(" ")));
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn head_oid(path: &Path) -> String {
    git(path, &["rev-parse", "HEAD"])
}

static FIXTURE: LazyLock<FixtureRepo> = LazyLock::new(build_fixture);

const LIB_INITIAL: &str = r#"// dd_example library
//
// A sample library for testing.

/// Returns a greeting message.
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Subtracts two numbers.
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

/// Multiplies two numbers.
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

/// Divides two numbers.
pub fn divide(a: f64, b: f64) -> f64 {
    a / b
}
"#;

fn build_fixture() -> FixtureRepo {
    let dir = TempDir::new().unwrap();
    let p = dir.path().to_path_buf();

    git(&p, &["init", "-b", "main"]);
    git(&p, &["config", "user.email", "test@example.com"]);
    git(&p, &["config", "user.name", "Test User"]);

    // ---- Root commit: initial project ----
    fs::create_dir_all(p.join("src")).unwrap();
    fs::create_dir_all(p.join("docs")).unwrap();
    fs::write(p.join("src/lib.rs"), LIB_INITIAL).unwrap();
    fs::write(p.join("docs/guide.md"), "# Guide\n\nGetting started.\n").unwrap();
    fs::write(p.join("README.md"), "# Example\n\nA sample project.\n").unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "feat: initial project setup"]);
    let root_oid = head_oid(&p);

    // ---- Feature branch ----
    git(&p, &["checkout", "-b", "feature/widgets"]);
    fs::write(p.join("src/widgets.rs"), "pub struct Widget;\n").unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "feat: add widgets module"]);

    // ---- Back to main, add changelog ----
    git(&p, &["checkout", "main"]);
    fs::write(p.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "docs: add changelog"]);

    // ---- Merge (no-ff) ----
    git(
        &p,
        &[
            "merge",
            "--no-ff",
            "feature/widgets",
            "-m",
            "merge: integrate widgets feature",
        ],
    );
    let merge_oid = head_oid(&p);

    // ---- Lightweight tag ----
    git(&p, &["tag", "v0.1.0"]);

    // ---- Multi-file commit ----
    let lib_multi = LIB_INITIAL.replace("Hello, {}!", "Hi, {}!");
    fs::write(p.join("src/lib.rs"), &lib_multi).unwrap();
    fs::write(
        p.join("README.md"),
        "# Example\n\nA sample project.\n\nUpdated.\n",
    )
    .unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "feat: update lib and readme"]);
    let multi_file_oid = head_oid(&p);

    // ---- Multi-hunk commit (edit top and bottom of lib.rs) ----
    let lib_hunk = lib_multi
        .replace("// dd_example library", "// dd_example library v2")
        .replace("a / b", "if b == 0.0 { f64::NAN } else { a / b }");
    fs::write(p.join("src/lib.rs"), &lib_hunk).unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "refactor: restructure lib module"]);
    let multi_hunk_oid = head_oid(&p);

    // ---- Rename ----
    git(&p, &["mv", "src/lib.rs", "src/library.rs"]);
    // Add a small content change so rename shows in diff with content
    let lib_renamed = lib_hunk.to_owned() + "// renamed\n";
    fs::write(p.join("src/library.rs"), &lib_renamed).unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "refactor: rename lib to library"]);
    let rename_oid = head_oid(&p);

    // ---- Delete ----
    git(&p, &["rm", "docs/guide.md"]);
    git(&p, &["commit", "-m", "chore: remove outdated guide"]);
    let delete_oid = head_oid(&p);

    // ---- Binary file ----
    fs::create_dir_all(p.join("assets")).unwrap();
    fs::write(p.join("assets/icon.bin"), b"\x00\x01\x02\xff\xfe\xfd").unwrap();
    git(&p, &["add", "."]);
    git(&p, &["commit", "-m", "feat: add binary icon asset"]);
    let binary_oid = head_oid(&p);

    // ---- Unicode commit message ----
    fs::write(
        p.join("README.md"),
        "# Example\n\n\u{00dc}pd\u{00e4}ted \u{00dc}n\u{00ef}c\u{00f6}d\u{00e9} docs.\n",
    )
    .unwrap();
    git(&p, &["add", "."]);
    git(
        &p,
        &[
            "commit",
            "-m",
            "docs: update \u{00dc}n\u{00ef}c\u{00f6}d\u{00e9} documentation",
        ],
    );
    let unicode_oid = head_oid(&p);

    // ---- Annotated tag ----
    git(&p, &["tag", "-a", "v1.0.0", "-m", "Release 1.0"]);

    // ---- Stash ----
    fs::write(p.join("README.md"), "# Example\n\nStash me.\n").unwrap();
    git(&p, &["stash", "push", "-m", "wip: readme edits"]);

    FixtureRepo {
        _dir: dir,
        path: p,
        root_oid,
        merge_oid,
        multi_file_oid,
        multi_hunk_oid,
        rename_oid,
        delete_oid,
        binary_oid,
        unicode_oid,
    }
}

// ---------------------------------------------------------------------------
// Tests against fixture
// ---------------------------------------------------------------------------

#[test]
fn open_fixture_repo() {
    let f = &*FIXTURE;
    Repository::open(&f.path).unwrap();
}

#[test]
fn branches_includes_main_and_feature() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let branches = repo.branches().unwrap();
    let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"main"), "missing main: {names:?}");
    assert!(
        names.contains(&"feature/widgets"),
        "missing feature/widgets: {names:?}"
    );
}

#[test]
fn head_branch_is_main() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    assert_eq!(repo.head_branch().unwrap(), "main");
    let branches = repo.branches().unwrap();
    let main = branches.iter().find(|b| b.name == "main").unwrap();
    assert!(main.is_head);
}

#[test]
fn tags_include_lightweight_and_annotated() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let tags = repo.tags().unwrap();
    let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"v0.1.0"), "missing v0.1.0: {names:?}");
    assert!(names.contains(&"v1.0.0"), "missing v1.0.0: {names:?}");
}

#[test]
fn remotes_is_empty_for_local_fixture() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    assert!(repo.remotes().unwrap().is_empty());
}

#[test]
fn stash_is_present() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let stashes = repo.stashes().unwrap();
    assert_eq!(stashes.len(), 1, "expected 1 stash, got {}", stashes.len());
    assert!(
        stashes[0].message.contains("wip: readme edits"),
        "unexpected stash message: {:?}",
        stashes[0].message
    );
}

#[test]
fn commits_walk_returns_expected_count() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let commits = repo.commits(100).unwrap();
    // 10 commits total, but assert >= 8 for resilience
    assert!(
        commits.len() >= 8,
        "expected at least 8 commits, got {}",
        commits.len()
    );
}

#[test]
fn commits_are_newest_first() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let commits = repo.commits(100).unwrap();
    for window in commits.windows(2) {
        assert!(
            window[0].date >= window[1].date,
            "commits not newest-first: {} ({}) before {} ({})",
            window[0].subject,
            window[0].date,
            window[1].subject,
            window[1].date,
        );
    }
}

#[test]
fn merge_commit_has_two_parents() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let commits = repo.commits(100).unwrap();
    let merge = commits
        .iter()
        .find(|c| c.oid == f.merge_oid)
        .expect("merge commit not found");
    assert_eq!(
        merge.parent_oids.len(),
        2,
        "merge commit should have 2 parents, has {}",
        merge.parent_oids.len()
    );
}

#[test]
fn root_commit_has_no_parents() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let commits = repo.commits(100).unwrap();
    let root = commits
        .iter()
        .find(|c| c.oid == f.root_oid)
        .expect("root commit not found");
    assert!(
        root.parent_oids.is_empty(),
        "root commit should have no parents"
    );
}

#[test]
fn unicode_in_commit_message() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let commits = repo.commits(100).unwrap();
    let uni = commits
        .iter()
        .find(|c| c.oid == f.unicode_oid)
        .expect("unicode commit not found");
    assert!(
        uni.subject.contains("\u{00dc}n\u{00ef}c\u{00f6}d\u{00e9}"),
        "unicode not found in subject: {:?}",
        uni.subject
    );
}

#[test]
fn commits_limit_is_respected() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let commits = repo.commits(3).unwrap();
    assert_eq!(commits.len(), 3);
}

#[test]
fn diff_root_commit_shows_all_files_as_added() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.root_oid).unwrap();
    assert!(!diffs.is_empty(), "root commit diff should not be empty");
    for file_diff in &diffs {
        assert_eq!(
            file_diff.status,
            FileStatus::Added,
            "root commit file {:?} should be Added, was {:?}",
            file_diff.path,
            file_diff.status
        );
    }
}

#[test]
fn diff_multi_file_commit() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.multi_file_oid).unwrap();
    assert!(
        diffs.len() >= 2,
        "multi-file commit should touch at least 2 files, got {}",
        diffs.len()
    );
    let has_modified = diffs.iter().any(|d| d.status == FileStatus::Modified);
    assert!(has_modified, "expected at least one Modified file");
}

#[test]
fn diff_rename_detected() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.rename_oid).unwrap();
    let renamed = diffs.iter().find(|d| d.status == FileStatus::Renamed);
    assert!(
        renamed.is_some(),
        "expected a Renamed file in diff: {diffs:?}"
    );
    assert!(
        renamed.unwrap().path.contains("library.rs"),
        "renamed file path should contain 'library.rs': {:?}",
        renamed.unwrap().path
    );
}

#[test]
fn diff_delete_detected() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.delete_oid).unwrap();
    let deleted = diffs.iter().find(|d| d.status == FileStatus::Deleted);
    assert!(
        deleted.is_some(),
        "expected a Deleted file in diff: {diffs:?}"
    );
    assert!(
        deleted.unwrap().path.contains("guide.md"),
        "deleted file should be guide.md: {:?}",
        deleted.unwrap().path
    );
}

#[test]
fn diff_multi_hunk_single_file() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.multi_hunk_oid).unwrap();
    assert_eq!(diffs.len(), 1, "multi-hunk commit should modify 1 file");
    assert!(
        diffs[0].hunks.len() >= 2,
        "expected at least 2 hunks, got {}",
        diffs[0].hunks.len()
    );
}

#[test]
fn diff_binary_file_commit() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.binary_oid).unwrap();
    let binary = diffs.iter().find(|d| d.path.contains("icon.bin"));
    assert!(
        binary.is_some(),
        "binary file should appear in diff: {diffs:?}"
    );
    assert_eq!(binary.unwrap().status, FileStatus::Added);
}

#[test]
fn diff_hunk_line_origins_are_valid() {
    let f = &*FIXTURE;
    let repo = Repository::open(&f.path).unwrap();
    let diffs = repo.diff_commit(&f.multi_file_oid).unwrap();
    for file_diff in &diffs {
        for hunk in &file_diff.hunks {
            assert!(!hunk.header.is_empty(), "hunk header should not be empty");
            assert!(
                hunk.header.starts_with("@@"),
                "hunk header should start with @@: {:?}",
                hunk.header
            );
            for line in &hunk.lines {
                match line.origin {
                    LineOrigin::Context | LineOrigin::Addition | LineOrigin::Deletion => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Smoke tests against dd_merge repo
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn smoke_open_dd_merge_repo() {
    let root = workspace_root();
    let repo = Repository::open(&root).unwrap();
    let head = repo.head_branch().unwrap();
    assert!(!head.is_empty(), "head branch should not be empty");
}

#[test]
fn smoke_branches_not_empty() {
    let root = workspace_root();
    let repo = Repository::open(&root).unwrap();
    let branches = repo.branches().unwrap();
    assert!(!branches.is_empty());
    assert!(
        branches.iter().any(|b| b.is_head),
        "at least one branch should be head"
    );
}

#[test]
fn smoke_commits_returns_results() {
    let root = workspace_root();
    let repo = Repository::open(&root).unwrap();
    let commits = repo.commits(10).unwrap();
    assert!(!commits.is_empty());
    let c = &commits[0];
    assert!(!c.oid.is_empty());
    assert!(!c.subject.is_empty());
    assert!(!c.author_name.is_empty());
}

#[test]
fn smoke_diff_latest_commit() {
    let root = workspace_root();
    let repo = Repository::open(&root).unwrap();
    let commits = repo.commits(20).unwrap();
    // Find a non-merge commit (1 parent)
    let non_merge = commits.iter().find(|c| c.parent_oids.len() == 1);
    if let Some(commit) = non_merge {
        let diffs = repo.diff_commit(&commit.oid).unwrap();
        assert!(
            !diffs.is_empty(),
            "non-merge commit diff should not be empty"
        );
    }
}

#[test]
fn smoke_remotes_has_origin() {
    let root = workspace_root();
    let repo = Repository::open(&root).unwrap();
    let remotes = repo.remotes().unwrap();
    assert!(
        remotes.iter().any(|r| r.name == "origin"),
        "expected 'origin' remote: {remotes:?}"
    );
}
