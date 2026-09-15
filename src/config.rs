//! Per-repo bootstrap config, read from `.herdr/bootstrap.toml` in the repo.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Config file candidates, relative to the repo/worktree root, in priority
/// order. TOML and YAML are both accepted — pick whichever you prefer; the
/// schema is identical. The first file that exists wins.
pub const CONFIG_PATHS: &[&str] = &[
    ".herdr/bootstrap.toml",
    ".herdr/bootstrap.yaml",
    ".herdr/bootstrap.yml",
];

/// Every config struct denies unknown fields: a typo like `pattern` for
/// `patterns` would otherwise deserialize to the default and silently do the
/// wrong thing, which is the most expensive kind of config bug to debug.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct GitConfig {
    /// Bring git up to date before the pre hooks run.
    #[serde(default)]
    pub update: bool,
    /// Override the update command. Defaults to `git fetch --all --prune`.
    #[serde(default)]
    pub command: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Hooks {
    /// Run first, before copy and install.
    #[serde(default)]
    pub pre: Vec<CommandConfig>,
    /// Run last, after copy and install.
    #[serde(default)]
    pub post: Vec<CommandConfig>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CopyConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Explicit relative paths to copy from the source repo. When set,
    /// gitignored discovery is disabled and exactly these files are copied
    /// (missing ones are skipped). Omit to use recursive discovery instead.
    #[serde(default)]
    pub files: Option<Vec<String>>,
    /// Filename globs used by recursive discovery (only when `files` is
    /// omitted). Discovery walks the source repo for **gitignored** files whose
    /// basename matches one of these globs and copies each to the same relative
    /// path in the worktree. Committed files (e.g. `.env.example`) are never
    /// copied because they aren't gitignored. Omit for the env-file defaults
    /// (`.env`, `.env.*`). `*` matches any sequence of characters.
    #[serde(default)]
    pub patterns: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InstallConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Custom detection rules, checked *before* the built-ins.
    /// Add any language here without touching Rust.
    #[serde(default)]
    pub rules: Vec<InstallRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallRule {
    /// File whose presence in the worktree triggers this rule (e.g. "go.mod").
    pub marker: String,
    /// Command to run when the marker is found (e.g. ["go", "mod", "download"]).
    pub command: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandConfig {
    /// argv, e.g. ["cargo", "build"]. First element is the program.
    pub command: Vec<String>,
}

/// Load the repo's `.herdr/bootstrap.{toml,yaml,yml}`. Each repo configures its
/// own bootstrap. A missing file is not an error — it just means "do nothing".
/// TOML and YAML are parsed from the same struct, so the format is chosen by
/// the file extension and nothing else changes.
pub fn load(repo: &Path) -> Result<Config> {
    let Some(path) = CONFIG_PATHS
        .iter()
        .map(|name| repo.join(name))
        .find(|path| path.is_file())
    else {
        println!(
            "[bootstrap] no .herdr/bootstrap.{{toml,yaml,yml}} in {}, nothing to do",
            repo.display()
        );
        return Ok(Config::default());
    };

    println!("[bootstrap] config:   {}", path.display());
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;

    let config: Config = match path.extension().and_then(|ext| ext.to_str()) {
        Some("yaml") | Some("yml") => {
            serde_yaml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?
        }
        _ => toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
    };
    Ok(config)
}
