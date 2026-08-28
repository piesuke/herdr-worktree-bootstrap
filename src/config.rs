//! Per-repo bootstrap config, read from `.herdr/bootstrap.toml` in the repo.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Path of the config file, relative to the repo/worktree root.
pub const CONFIG_PATH: &str = ".herdr/bootstrap.toml";

#[derive(Deserialize, Default)]
pub struct Config {
    /// Update git (e.g. fetch) before anything else.
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub copy: CopyConfig,
    #[serde(default)]
    pub install: InstallConfig,
    /// Commands run before/after the copy + install phases.
    #[serde(default)]
    pub hooks: Hooks,
}

#[derive(Deserialize, Default)]
pub struct GitConfig {
    /// Bring git up to date before the pre hooks run.
    #[serde(default)]
    pub update: bool,
    /// Override the update command. Defaults to `git fetch --all --prune`.
    #[serde(default)]
    pub command: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
pub struct Hooks {
    /// Run first, before copy and install.
    #[serde(default)]
    pub pre: Vec<CommandConfig>,
    /// Run last, after copy and install.
    #[serde(default)]
    pub post: Vec<CommandConfig>,
}

#[derive(Deserialize, Default)]
pub struct CopyConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Files to copy. Omit to use the built-in default env-file list;
    /// set to an explicit list (possibly empty) to override it.
    #[serde(default)]
    pub files: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
pub struct InstallConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Custom detection rules, checked *before* the built-ins.
    /// Add any language here without touching Rust.
    #[serde(default)]
    pub rules: Vec<InstallRule>,
}

#[derive(Deserialize)]
pub struct InstallRule {
    /// File whose presence in the worktree triggers this rule (e.g. "go.mod").
    pub marker: String,
    /// Command to run when the marker is found (e.g. ["go", "mod", "download"]).
    pub command: Vec<String>,
}

#[derive(Deserialize)]
pub struct CommandConfig {
    /// argv, e.g. ["cargo", "build"]. First element is the program.
    pub command: Vec<String>,
}

/// Load the repo's `.herdr/bootstrap.toml`. Each repo configures its own
/// bootstrap. A missing file is not an error — it just means "do nothing".
pub fn load(repo: &Path) -> Result<Config> {
    let path = repo.join(CONFIG_PATH);
    if !path.is_file() {
        println!("[bootstrap] no {}, nothing to do", path.display());
        return Ok(Config::default());
    }
    println!("[bootstrap] config:   {}", path.display());
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: Config =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(config)
}
