//! Entry point: parse the herdr event payload, locate the repo's config, and
//! hand off to [`herdr_worktree_bootstrap::run`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use herdr_worktree_bootstrap::{config, event::Event, run};

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

    // Config is owned by the repo: read `.herdr/worktree-bootstrap.toml` from the source
    // repo (falls back to the new worktree, which has the same committed copy).
    let config_dir = source.as_deref().unwrap_or(worktree);
    let config = config::load(config_dir).context("failed to load bootstrap config")?;

    run(worktree, source.as_deref(), &config)?;

    println!("[bootstrap] done");
    Ok(())
}
