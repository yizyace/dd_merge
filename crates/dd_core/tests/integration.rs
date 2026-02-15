use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use dd_core::{AppState, Session};

#[test]
fn session_roundtrip_multiple_repos() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("session.json");

    let mut state = AppState::default();
    state.add_repo(PathBuf::from("/projects/alpha"));
    state.add_repo(PathBuf::from("/projects/beta"));
    state.add_repo(PathBuf::from("/projects/gamma"));
    state.active_tab = 1;

    Session::save_to(&path, &state).unwrap();
    let loaded = Session::load_from(&path).unwrap().unwrap();

    assert_eq!(loaded.repos.len(), 3);
    assert_eq!(loaded.repos[0].name, "alpha");
    assert_eq!(loaded.repos[0].path, PathBuf::from("/projects/alpha"));
    assert_eq!(loaded.repos[1].name, "beta");
    assert_eq!(loaded.repos[2].name, "gamma");
    assert_eq!(loaded.active_tab, 1);
}

#[test]
fn session_roundtrip_empty_state() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("session.json");

    let state = AppState::default();
    Session::save_to(&path, &state).unwrap();
    let loaded = Session::load_from(&path).unwrap().unwrap();

    assert!(loaded.repos.is_empty());
    assert_eq!(loaded.active_tab, 0);
}

#[test]
fn session_load_missing_file_returns_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.json");
    let result = Session::load_from(&path).unwrap();
    assert!(result.is_none());
}

#[test]
fn session_load_corrupt_json_returns_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("corrupt.json");
    fs::write(&path, "not valid json {{{").unwrap();
    let result = Session::load_from(&path);
    assert!(result.is_err());
}

#[test]
fn state_add_remove_sequence() {
    let mut state = AppState::default();
    state.add_repo(PathBuf::from("/a"));
    state.add_repo(PathBuf::from("/b"));
    state.add_repo(PathBuf::from("/c"));
    assert_eq!(state.active_tab, 2); // active = last added

    // Remove middle (/b at index 1)
    state.remove_repo(1);
    assert_eq!(state.repos.len(), 2);
    assert_eq!(state.repos[0].name, "a");
    assert_eq!(state.repos[1].name, "c");
    // active_tab was 2, now clamped to repos.len()-1 = 1
    assert_eq!(state.active_tab, 1);

    // Remove first (/a at index 0)
    state.remove_repo(0);
    assert_eq!(state.repos.len(), 1);
    assert_eq!(state.repos[0].name, "c");
    assert_eq!(state.active_tab, 0);
}
