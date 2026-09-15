//! Shared helpers for the integration tests.
//!
//! Each integration test file is its own crate and compiles this module
//! separately, so helpers one file doesn't use would otherwise trip
//! `dead_code` — which CI escalates to an error.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// A throwaway directory. Deleted when dropped, so tests never share state.
pub struct Dir(TempDir);

impl Dir {
    pub fn new() -> Self {
        Dir(tempfile::tempdir().expect("creating tempdir"))
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// Write `contents` to `rel`, creating parent directories as needed.
    pub fn write(&self, rel: &str, contents: &str) -> PathBuf {
        let path = self.0.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("creating parent dirs");
        }
        std::fs::write(&path, contents).expect("writing file");
        path
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.0.path().join(rel))
            .unwrap_or_else(|e| panic!("reading {rel}: {e}"))
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.0.path().join(rel).exists()
    }

    /// Lines of a file written by hooks, with blanks trimmed. Missing file =
    /// no lines, so a test can assert a phase never ran.
    pub fn log_lines(&self, rel: &str) -> Vec<String> {
        match std::fs::read_to_string(self.0.path().join(rel)) {
            Ok(text) => text.lines().map(str::to_string).collect(),
            Err(_) => Vec::new(),
        }
    }
}

/// Run a git subcommand in `dir`, panicking on failure.
pub fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawning git {args:?}: {e}"));
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An initialized git repository with everything currently present committed.
///
/// `core.excludesFile` is pointed at `/dev/null` so the developer's global
/// gitignore can't change which files the discovery tests see.
pub fn init_repo(dir: &Dir) {
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["config", "core.excludesFile", "/dev/null"]);
    commit_all(dir, "init");
}

/// Stage and commit everything that isn't ignored.
pub fn commit_all(dir: &Dir, message: &str) {
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            message,
        ],
    );
}
