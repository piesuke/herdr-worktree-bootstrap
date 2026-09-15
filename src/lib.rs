//! Bootstrap a freshly created git worktree: copy gitignored files, install
//! dependencies, and run per-repo hooks.
//!
//! The binary (`herdr-worktree-init`) is a thin wrapper that parses the herdr
//! event payload and calls [`run`]. Everything that decides *what happens* lives
//! here so it can be exercised from tests without herdr in the loop.

pub mod bootstrap;
pub mod config;
pub mod event;

use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;

/// Run the whole bootstrap lifecycle against `worktree`.
///
/// Phases run in a fixed order — **git update → pre hooks → copy → install →
/// post hooks** — and the first failure aborts the rest (fail-fast), leaving the
/// worktree partially bootstrapped rather than silently continuing past a broken
/// step.
///
/// `source` is the repo the worktree was derived from. It is only needed by the
/// copy phase, which reads the gitignored files a fresh checkout won't have; the
/// other phases operate entirely inside `worktree`.
pub fn run(worktree: &Path, source: Option<&Path>, config: &Config) -> Result<()> {
    // Phase 0: bring git up to date, before anything else runs.
    if config.git.update {
        bootstrap::git_update(worktree, config.git.command.as_deref())?;
    }

    bootstrap::run_hooks(worktree, &config.hooks.pre)?;

    if config.copy.enabled {
        let src = source.context("copy is enabled but the event has no source repo_root")?;
        match config.copy.files.as_deref() {
            Some(files) => bootstrap::copy_files(src, worktree, files)?,
            None => bootstrap::copy_gitignored(src, worktree, config.copy.patterns.as_deref())?,
        }
    }

    if config.install.enabled {
        bootstrap::install_deps(
            worktree,
            &config.install.rules,
            config.install.dirs.as_deref(),
        )?;
    }

    // Post hooks: run last, after copy and install.
    bootstrap::run_hooks(worktree, &config.hooks.post)?;

    Ok(())
}
