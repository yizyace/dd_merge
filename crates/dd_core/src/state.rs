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

    pub fn remove_repo(&mut self, index: usize) {
        if index < self.repos.len() {
            self.repos.remove(index);
            if self.active_tab >= self.repos.len() && !self.repos.is_empty() {
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
}
