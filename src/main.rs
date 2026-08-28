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

    let config = config::load().context("failed to load plugin config")?;

    if config.copy.enabled {
        let src = source
            .as_deref()
            .context("copy is enabled but the event has no source repo_root")?;
        bootstrap::copy_files(src, worktree, &config.copy.files)?;
    }

    if config.install.enabled {
        bootstrap::install_deps(worktree, &config.install.rules)?;
    }

    for cmd in &config.commands {
        bootstrap::run_command(worktree, &cmd.command)?;
    }

    println!("[bootstrap] done");
    Ok(())
}
