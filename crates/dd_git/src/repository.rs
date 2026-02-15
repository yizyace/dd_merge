use std::path::Path;

use anyhow::{Context, Result};
use gix::bstr::ByteSlice;

use crate::commit::CommitInfo;
use crate::diff::FileDiff;
use crate::types::{BranchInfo, RemoteInfo, StashInfo, TagInfo};

pub struct Repository {
    inner: gix::Repository,
}

impl Repository {
    pub fn open(path: &Path) -> Result<Self> {
        let inner = gix::open(path)
            .with_context(|| format!("failed to open git repository at {}", path.display()))?;
        Ok(Self { inner })
    }

    pub fn head_branch(&self) -> Result<String> {
        let head = self.inner.head()?;
        if let Some(name) = head.referent_name() {
            Ok(name.shorten().to_string())
        } else {
            Ok("HEAD (detached)".to_string())
        }
    }

    pub fn branches(&self) -> Result<Vec<BranchInfo>> {
        let head_name = self.head_branch().unwrap_or_default();
        let refs = self.inner.references()?;
        let mut branches = Vec::new();
        for reference in refs.local_branches()?.flatten() {
            let name = reference.name().shorten().to_string();
            let is_head = name == head_name;
            branches.push(BranchInfo { name, is_head });
        }
        branches.sort_by(|a, b| b.is_head.cmp(&a.is_head).then_with(|| a.name.cmp(&b.name)));
        Ok(branches)
    }

    pub fn remotes(&self) -> Result<Vec<RemoteInfo>> {
        let names = self.inner.remote_names();
        let mut remotes: Vec<RemoteInfo> = names
            .iter()
            .map(|name| RemoteInfo {
                name: name.to_string(),
            })
            .collect();
        remotes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(remotes)
    }

    pub fn tags(&self) -> Result<Vec<TagInfo>> {
        let refs = self.inner.references()?;
        let mut tags = Vec::new();
        for reference in refs.tags()?.flatten() {
            let name = reference.name().shorten().to_string();
            tags.push(TagInfo { name });
        }
        tags.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tags)
    }

    pub fn stashes(&self) -> Result<Vec<StashInfo>> {
        let stash_ref = self.inner.try_find_reference("refs/stash")?;
        let Some(stash_ref) = stash_ref else {
            return Ok(Vec::new());
        };
        let mut stashes = Vec::new();
        let mut log = stash_ref.log_iter();
        if let Some(log) = log.all()? {
            for entry in log {
                let entry = entry?;
                stashes.push(StashInfo {
                    message: entry.message.to_string(),
                });
            }
        }
        stashes.reverse();
        Ok(stashes)
    }

    pub fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>> {
        let head_id = self.inner.head_id()?;
        let walk = self
            .inner
            .rev_walk([head_id])
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                Default::default(),
            ))
            .all()?;

        let mut commits = Vec::new();
        for info in walk {
            if commits.len() >= limit {
                break;
            }
            let info = info?;
            let commit = info.object()?;
            let author = commit.author()?;
            let message = commit.message()?;
            let parent_oids: Vec<String> = info
                .parent_ids
                .iter()
                .map(|id| id.to_hex().to_string())
                .collect();

            let oid = info.id.to_hex().to_string();
            let short_oid = info.id.to_hex_with_len(7).to_string();

            commits.push(CommitInfo {
                oid,
                short_oid,
                author_name: author.name.to_string(),
                author_email: author.email.to_string(),
                date: author.time.seconds,
                subject: message.title.to_str_lossy().trim().to_string(),
                body: message
                    .body
                    .map(|b| b.to_str_lossy().trim().to_string())
                    .unwrap_or_default(),
                parent_oids,
            });
        }
        Ok(commits)
    }

    pub fn is_dirty(&self) -> Result<bool> {
        // Check tracked changes (staged + unstaged modifications) first via
        // the fast built-in check which skips the directory walk.
        if self.inner.is_dirty()? {
            return Ok(true);
        }
        // Also check for untracked files via the index-worktree iterator
        // with directory walk enabled.
        let iter = self
            .inner
            .status(gix::progress::Discard)?
            .into_index_worktree_iter(Vec::<gix::bstr::BString>::new())?;
        for item in iter {
            let item = item?;
            if matches!(
                item,
                gix::status::index_worktree::Item::DirectoryContents { .. }
            ) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn diff_commit(&self, oid: &str) -> Result<Vec<FileDiff>> {
        let workdir = self
            .inner
            .work_dir()
            .context("repository has no working directory")?;
        crate::diff::diff_commit(workdir, oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(path: &std::path::Path, args: &[&str]) {
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

    fn init_test_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        git(path, &["init", "-b", "main"]);
        git(path, &["config", "user.email", "test@test.com"]);
        git(path, &["config", "user.name", "Test"]);
        std::fs::write(path.join("file.txt"), "hello").unwrap();
        git(path, &["add", "."]);
        git(path, &["commit", "-m", "initial"]);
        let repo = Repository::open(path).unwrap();
        (dir, repo)
    }

    #[test]
    fn test_open_valid_repo() {
        let (_dir, _repo) = init_test_repo();
    }

    #[test]
    fn test_open_non_git_dir_fails() {
        let dir = TempDir::new().unwrap();
        let result = Repository::open(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_head_branch() {
        let (_dir, repo) = init_test_repo();
        let head = repo.head_branch().unwrap();
        assert_eq!(head, "main");
    }

    #[test]
    fn test_branches() {
        let (_dir, repo) = init_test_repo();
        let branches = repo.branches().unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_head);
    }

    #[test]
    fn test_tags_empty() {
        let (_dir, repo) = init_test_repo();
        let tags = repo.tags().unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_remotes_empty() {
        let (_dir, repo) = init_test_repo();
        let remotes = repo.remotes().unwrap();
        assert!(remotes.is_empty());
    }

    #[test]
    fn test_stashes_empty() {
        let (_dir, repo) = init_test_repo();
        let stashes = repo.stashes().unwrap();
        assert!(stashes.is_empty());
    }

    fn init_test_repo_with_commits(count: usize) -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        git(path, &["init", "-b", "main"]);
        git(path, &["config", "user.email", "test@test.com"]);
        git(path, &["config", "user.name", "Test User"]);
        for i in 0..count {
            std::fs::write(path.join("file.txt"), format!("content {i}")).unwrap();
            git(path, &["add", "."]);
            git(path, &["commit", "-m", &format!("commit {i}")]);
        }
        let repo = Repository::open(path).unwrap();
        (dir, repo)
    }

    #[test]
    fn test_commits_returns_correct_count() {
        let (_dir, repo) = init_test_repo_with_commits(5);
        let commits = repo.commits(3).unwrap();
        assert_eq!(commits.len(), 3);
    }

    #[test]
    fn test_commits_newest_first() {
        let (_dir, repo) = init_test_repo_with_commits(5);
        let commits = repo.commits(5).unwrap();
        assert_eq!(commits.len(), 5);
        assert_eq!(commits[0].subject, "commit 4");
        assert_eq!(commits[4].subject, "commit 0");
    }

    #[test]
    fn test_commit_info_fields() {
        let (_dir, repo) = init_test_repo_with_commits(1);
        let commits = repo.commits(1).unwrap();
        let commit = &commits[0];
        assert_eq!(commit.subject, "commit 0");
        assert_eq!(commit.author_name, "Test User");
        assert_eq!(commit.author_email, "test@test.com");
        assert_eq!(commit.short_oid.len(), 7);
        assert!(commit.parent_oids.is_empty()); // first commit has no parent
    }

    #[test]
    fn test_commits_have_parent_oids() {
        let (_dir, repo) = init_test_repo_with_commits(2);
        let commits = repo.commits(2).unwrap();
        assert_eq!(commits[0].parent_oids.len(), 1);
        assert_eq!(commits[0].parent_oids[0], commits[1].oid);
    }

    #[test]
    fn test_is_dirty_clean_repo() {
        let (_dir, repo) = init_test_repo();
        assert!(!repo.is_dirty().unwrap());
    }

    #[test]
    fn test_is_dirty_unstaged_modification() {
        let (dir, repo) = init_test_repo();
        std::fs::write(dir.path().join("file.txt"), "modified").unwrap();
        assert!(repo.is_dirty().unwrap());
    }

    #[test]
    fn test_is_dirty_staged_change() {
        let (dir, repo) = init_test_repo();
        std::fs::write(dir.path().join("file.txt"), "staged").unwrap();
        git(dir.path(), &["add", "file.txt"]);
        // Re-open to pick up index changes
        let repo = Repository::open(dir.path()).unwrap();
        assert!(repo.is_dirty().unwrap());
    }

    #[test]
    fn test_is_dirty_untracked_file() {
        let (dir, _repo) = init_test_repo();
        std::fs::write(dir.path().join("new_file.txt"), "untracked").unwrap();
        // Re-open so the repo picks up the new working directory state
        let repo = Repository::open(dir.path()).unwrap();
        assert!(repo.is_dirty().unwrap());
    }

    #[test]
    fn test_diff_commit_shows_modification() {
        let (_dir, repo) = init_test_repo_with_commits(2);
        let commits = repo.commits(2).unwrap();
        let diffs = repo.diff_commit(&commits[0].oid).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "file.txt");
        assert!(!diffs[0].hunks.is_empty());
        // Should have both additions and deletions
        let has_addition = diffs[0].hunks[0]
            .lines
            .iter()
            .any(|l| l.origin == crate::diff::LineOrigin::Addition);
        let has_deletion = diffs[0].hunks[0]
            .lines
            .iter()
            .any(|l| l.origin == crate::diff::LineOrigin::Deletion);
        assert!(has_addition);
        assert!(has_deletion);
    }

    #[test]
    fn test_diff_root_commit() {
        let (_dir, repo) = init_test_repo_with_commits(1);
        let commits = repo.commits(1).unwrap();
        let diffs = repo.diff_commit(&commits[0].oid).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "file.txt");
    }
}
