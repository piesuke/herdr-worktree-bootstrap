//! Plugin config, read from `herdr-plugin.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub copy: CopyConfig,
    #[serde(default)]
    pub install: InstallConfig,
    /// Arbitrary commands to run in the new worktree, in order.
    #[serde(default)]
    pub commands: Vec<CommandConfig>,
}

#[derive(Deserialize, Default)]
pub struct CopyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub files: Vec<String>,
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

/// Load the config: env var first, then walk up from the executable.
pub fn load() -> Result<Config> {
    let path = find().context("could not locate herdr-plugin.toml")?;
    println!("[bootstrap] config:   {}", path.display());
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: Config =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(config)
}

fn find() -> Option<PathBuf> {
    // 1. env override(s) pointing at the plugin dir.
    for var in ["HERDR_PLUGIN_DIR", "HERDR_PLUGIN_ROOT"] {
        if let Ok(dir) = std::env::var(var) {
            let candidate = Path::new(&dir).join("herdr-plugin.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    // 2. walk up from the executable location.
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            let candidate = ancestor.join("herdr-plugin.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    // 3. fall back to the current working directory.
    let cwd = Path::new("herdr-plugin.toml");
    cwd.is_file().then(|| cwd.to_path_buf())
}
