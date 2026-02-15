use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::state::AppState;

fn session_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("could not determine config directory")?;
    Ok(config_dir.join("dd_merge").join("session.json"))
}

pub struct Session;

impl Session {
    pub fn save(state: &AppState) -> Result<()> {
        Self::save_to(&session_path()?, state)
    }

    pub fn load() -> Result<Option<AppState>> {
        Self::load_from(&session_path()?)
    }

    pub fn save_to(path: &std::path::Path, state: &AppState) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(state)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from(path: &std::path::Path) -> Result<Option<AppState>> {
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(path)?;
        let state: AppState = serde_json::from_str(&json)?;
        Ok(Some(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("session.json");

        let mut state = AppState::default();
        state.add_repo(PathBuf::from("/tmp/repo1"));
        state.add_repo(PathBuf::from("/tmp/repo2"));
        state.active_tab = 1;

        Session::save_to(&path, &state).unwrap();
        let loaded = Session::load_from(&path).unwrap().unwrap();

        assert_eq!(loaded.repos.len(), 2);
        assert_eq!(loaded.repos[0].name, "repo1");
        assert_eq!(loaded.repos[1].name, "repo2");
        assert_eq!(loaded.active_tab, 1);
    }

    #[test]
    fn test_load_returns_none_when_no_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        let result = Session::load_from(&path).unwrap();
        assert!(result.is_none());
    }
}
