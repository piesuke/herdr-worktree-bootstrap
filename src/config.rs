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
    #[serde(default)]
    pub manager: PackageManager,
}

#[derive(Deserialize)]
pub struct CommandConfig {
    /// argv, e.g. ["cargo", "build"]. First element is the program.
    pub command: Vec<String>,
}

#[derive(Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    /// Detect from lockfiles in the worktree.
    #[default]
    Auto,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Cargo,
    None,
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
