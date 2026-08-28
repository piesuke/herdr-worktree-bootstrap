//! The three bootstrap phases: copy, install, run.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::config::PackageManager;

/// Copy files (e.g. `.env`) from the source repo into the worktree.
pub fn copy_files(source: &Path, worktree: &Path, files: &[String]) -> Result<()> {
    for file in files {
        let src = source.join(file);
        let dst = worktree.join(file);
        if !src.exists() {
            println!("[copy] skip (missing in source): {file}");
            continue;
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copying {} -> {}", src.display(), dst.display()))?;
        println!("[copy] {file}");
    }
    Ok(())
}

/// Install dependencies inside the worktree.
pub fn install_deps(worktree: &Path, manager: PackageManager) -> Result<()> {
    let manager = match manager {
        PackageManager::Auto => detect_manager(worktree),
        other => other,
    };

    let argv: &[&str] = match manager {
        PackageManager::Npm => &["npm", "ci"],
        PackageManager::Pnpm => &["pnpm", "install", "--frozen-lockfile"],
        PackageManager::Yarn => &["yarn", "install", "--frozen-lockfile"],
        PackageManager::Bun => &["bun", "install"],
        PackageManager::Cargo => &["cargo", "fetch"],
        PackageManager::None | PackageManager::Auto => {
            println!("[install] no package manager detected, skipping");
            return Ok(());
        }
    };

    let owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    run_command(worktree, &owned)
}

fn detect_manager(worktree: &Path) -> PackageManager {
    let has = |f: &str| worktree.join(f).exists();
    if has("bun.lockb") {
        PackageManager::Bun
    } else if has("pnpm-lock.yaml") {
        PackageManager::Pnpm
    } else if has("yarn.lock") {
        PackageManager::Yarn
    } else if has("package-lock.json") {
        PackageManager::Npm
    } else if has("Cargo.toml") {
        PackageManager::Cargo
    } else {
        PackageManager::None
    }
}

/// Run one command inside the worktree. Non-zero exit aborts (returns Err).
pub fn run_command(worktree: &Path, argv: &[String]) -> Result<()> {
    let Some((program, rest)) = argv.split_first() else {
        bail!("empty command in config");
    };
    println!("[run] {}", argv.join(" "));

    let status = Command::new(program)
        .args(rest)
        .current_dir(worktree)
        .status()
        .with_context(|| format!("spawning `{program}`"))?;

    if !status.success() {
        bail!("`{}` exited with {}", argv.join(" "), status);
    }
    Ok(())
}
