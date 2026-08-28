//! The three bootstrap phases: copy, install, run.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::InstallRule;

/// Built-in detection rules, checked *after* any user-defined rules.
/// The first marker present in the worktree wins, so more specific lockfiles
/// are listed before generic manifests.
const BUILTIN_RULES: &[(&str, &[&str])] = &[
    // JavaScript / TypeScript
    ("bun.lockb", &["bun", "install"]),
    ("pnpm-lock.yaml", &["pnpm", "install", "--frozen-lockfile"]),
    ("yarn.lock", &["yarn", "install", "--frozen-lockfile"]),
    ("package-lock.json", &["npm", "ci"]),
    ("package.json", &["npm", "install"]),
    ("deno.lock", &["deno", "install"]),
    // Rust
    ("Cargo.toml", &["cargo", "fetch"]),
    // Go
    ("go.mod", &["go", "mod", "download"]),
    // Python
    ("uv.lock", &["uv", "sync"]),
    ("poetry.lock", &["poetry", "install"]),
    ("Pipfile.lock", &["pipenv", "install", "--dev"]),
    ("requirements.txt", &["pip", "install", "-r", "requirements.txt"]),
    // Ruby
    ("Gemfile.lock", &["bundle", "install"]),
    // PHP
    ("composer.json", &["composer", "install"]),
    // Java / Kotlin
    ("pom.xml", &["mvn", "install", "-DskipTests"]),
    ("build.gradle", &["gradle", "build", "-x", "test"]),
    // Elixir
    ("mix.exs", &["mix", "deps.get"]),
    // Dart / Flutter
    ("pubspec.yaml", &["dart", "pub", "get"]),
];

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
///
/// User-defined `rules` are checked first (so they can add languages or
/// override a built-in), then the built-in table. The first marker that
/// exists in the worktree wins.
pub fn install_deps(worktree: &Path, rules: &[InstallRule]) -> Result<()> {
    for rule in rules {
        if worktree.join(&rule.marker).exists() {
            return run_command(worktree, &rule.command);
        }
    }
    for (marker, argv) in BUILTIN_RULES {
        if worktree.join(marker).exists() {
            let owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
            return run_command(worktree, &owned);
        }
    }
    println!("[install] no matching install rule, skipping");
    Ok(())
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
