use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTab {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub repos: Vec<RepoTab>,
    pub active_tab: usize,
}

impl AppState {
    pub fn add_repo(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        self.repos.push(RepoTab { path, name });
        self.active_tab = self.repos.len() - 1;
    }

    pub fn reorder_repos(&mut self, from: usize, to: usize) {
        let len = self.repos.len();
        if from == to || from >= len || to >= len {
            return;
        }
        let active_path = self.repos.get(self.active_tab).map(|r| r.path.clone());
        let repo = self.repos.remove(from);
        self.repos.insert(to, repo);
        if let Some(path) = active_path {
            if let Some(pos) = self.repos.iter().position(|r| r.path == path) {
                self.active_tab = pos;
            }
        }
    }

    pub fn remove_repo(&mut self, index: usize) {
        if index < self.repos.len() {
            self.repos.remove(index);
            if self.repos.is_empty() {
                self.active_tab = 0;
            } else if self.active_tab >= self.repos.len() {
                self.active_tab = self.repos.len() - 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.repos.is_empty());
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_add_repo() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/my_repo"));
        assert_eq!(state.repos.len(), 1);
        assert_eq!(state.repos[0].name, "my_repo");
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_add_multiple_repos() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/repo1"));
        state.add_repo(PathBuf::from("/tmp/repo2"));
        assert_eq!(state.repos.len(), 2);
        assert_eq!(state.active_tab, 1);
    }

    #[test]
    fn test_remove_repo() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/repo1"));
        state.add_repo(PathBuf::from("/tmp/repo2"));
        state.remove_repo(0);
        assert_eq!(state.repos.len(), 1);
        assert_eq!(state.repos[0].name, "repo2");
    }

    #[test]
    fn test_remove_repo_out_of_bounds() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/repo1"));
        state.add_repo(PathBuf::from("/tmp/repo2"));
        state.remove_repo(99);
        assert_eq!(state.repos.len(), 2);
        assert_eq!(state.active_tab, 1);
    }

    #[test]
    fn test_reorder_repos_forward() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/a"));
        state.add_repo(PathBuf::from("/tmp/b"));
        state.add_repo(PathBuf::from("/tmp/c"));
        state.active_tab = 0; // point at "a"
        state.reorder_repos(0, 2);
        assert_eq!(state.repos[0].name, "b");
        assert_eq!(state.repos[1].name, "c");
        assert_eq!(state.repos[2].name, "a");
        assert_eq!(state.active_tab, 2); // follows "a"
    }

    #[test]
    fn test_reorder_repos_backward() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/a"));
        state.add_repo(PathBuf::from("/tmp/b"));
        state.add_repo(PathBuf::from("/tmp/c"));
        state.active_tab = 2; // point at "c"
        state.reorder_repos(2, 0);
        assert_eq!(state.repos[0].name, "c");
        assert_eq!(state.repos[1].name, "a");
        assert_eq!(state.repos[2].name, "b");
        assert_eq!(state.active_tab, 0); // follows "c"
    }

    #[test]
    fn test_reorder_repos_same_index_noop() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/a"));
        state.add_repo(PathBuf::from("/tmp/b"));
        state.active_tab = 0;
        state.reorder_repos(1, 1);
        assert_eq!(state.repos[0].name, "a");
        assert_eq!(state.repos[1].name, "b");
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_reorder_repos_out_of_bounds_noop() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/a"));
        state.add_repo(PathBuf::from("/tmp/b"));
        state.active_tab = 0;
        state.reorder_repos(0, 99);
        assert_eq!(state.repos[0].name, "a");
        assert_eq!(state.repos[1].name, "b");
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_reorder_repos_active_tab_follows() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/a"));
        state.add_repo(PathBuf::from("/tmp/b"));
        state.add_repo(PathBuf::from("/tmp/c"));
        state.active_tab = 1; // point at "b"
        state.reorder_repos(0, 2); // move "a" to end
        assert_eq!(state.repos[0].name, "b");
        assert_eq!(state.active_tab, 0); // "b" is now at 0
    }

    #[test]
    fn test_remove_all_repos() {
        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/repo1"));
        state.add_repo(PathBuf::from("/tmp/repo2"));
        state.remove_repo(1);
        state.remove_repo(0);
        assert!(state.repos.is_empty());
        assert_eq!(state.active_tab, 0);
    }
}
