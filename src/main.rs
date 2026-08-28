mod bootstrap;
mod config;
mod event;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::event::Event;

fn main() -> Result<()> {
    let event_json =
        std::env::var("HERDR_PLUGIN_EVENT_JSON").context("HERDR_PLUGIN_EVENT_JSON is not set")?;
    let event: Event =
        serde_json::from_str(&event_json).context("failed to parse HERDR_PLUGIN_EVENT_JSON")?;

    let worktree = Path::new(&event.data.worktree.path);
    println!("[bootstrap] worktree: {}", worktree.display());
    println!("[bootstrap] branch:   {}", event.data.worktree.branch);

    let source = event
        .data
        .workspace
        .worktree
        .as_ref()
        .map(|w| PathBuf::from(&w.repo_root));
    if let Some(src) = &source {
        println!("[bootstrap] source:   {}", src.display());
    }

    // Config is owned by the repo: read `.herdr/bootstrap.toml` from the source
    // repo (falls back to the new worktree, which has the same committed copy).
    let config_dir = source.as_deref().unwrap_or(worktree);
    let config = config::load(config_dir).context("failed to load bootstrap config")?;

    // Pre hooks: run first, before anything else.
    bootstrap::run_hooks(worktree, &config.hooks.pre)?;

    // Phase 1: copy files (e.g. .env) from the source repo into the worktree.
    if config.copy.enabled {
        let src = source
            .as_deref()
            .context("copy is enabled but the event has no source repo_root")?;
        bootstrap::copy_files(src, worktree, &config.copy.files)?;
    }

    // Phase 2: install dependencies inside the worktree.
    if config.install.enabled {
        bootstrap::install_deps(worktree, &config.install.rules)?;
    }

    // Post hooks: run last, after copy and install.
    bootstrap::run_hooks(worktree, &config.hooks.post)?;

    println!("[bootstrap] done");
    Ok(())
}
