//! The three bootstrap phases: copy, install, run.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{CommandConfig, InstallRule};

/// Default git update command when `[git] update = true` and no override given.
const DEFAULT_GIT_UPDATE: &[&str] = &["git", "fetch", "--all", "--prune"];

/// Default files copied when `[copy]` is enabled but `files` is omitted.
const DEFAULT_COPY_FILES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.development.local",
    ".env.test.local",
    ".env.production.local",
];

/// Built-in install rules, checked *after* any user-defined rules. The first
/// matching marker wins, so more specific lockfiles precede generic manifests.
/// A marker of the form `*.ext` matches any file with that extension in the
/// worktree root. Covers the ~top-20 languages by usage that have a package /
/// dependency manager.
const BUILTIN_RULES: &[(&str, &[&str])] = &[
    // JavaScript / TypeScript / Node
    ("bun.lockb", &["bun", "install"]),
    ("pnpm-lock.yaml", &["pnpm", "install", "--frozen-lockfile"]),
    ("yarn.lock", &["yarn", "install", "--frozen-lockfile"]),
    ("package-lock.json", &["npm", "ci"]),
    ("deno.lock", &["deno", "install"]),
    ("package.json", &["npm", "install"]),
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
    ("Gemfile", &["bundle", "install"]),
    // PHP
    ("composer.json", &["composer", "install"]),
    // Java / Kotlin / Scala (JVM)
    ("pom.xml", &["mvn", "install", "-DskipTests"]),
    ("build.gradle.kts", &["gradle", "build", "-x", "test"]),
    ("build.gradle", &["gradle", "build", "-x", "test"]),
    ("build.sbt", &["sbt", "update"]),
    // C# / .NET
    ("*.sln", &["dotnet", "restore"]),
    ("*.csproj", &["dotnet", "restore"]),
    // C / C++
    ("vcpkg.json", &["vcpkg", "install"]),
    ("conanfile.txt", &["conan", "install", "."]),
    ("conanfile.py", &["conan", "install", "."]),
    // Swift / Objective-C
    ("Package.swift", &["swift", "package", "resolve"]),
    ("Podfile", &["pod", "install"]),
    // Dart / Flutter
    ("pubspec.yaml", &["dart", "pub", "get"]),
    // Elixir
    ("mix.exs", &["mix", "deps.get"]),
    // Erlang
    ("rebar.config", &["rebar3", "get-deps"]),
    // Haskell
    ("stack.yaml", &["stack", "build", "--only-dependencies"]),
    ("cabal.project", &["cabal", "build", "--only-dependencies"]),
    // R
    ("renv.lock", &["Rscript", "-e", "renv::restore(prompt = FALSE)"]),
    // Perl
    ("cpanfile", &["cpanm", "--installdeps", "."]),
    // Clojure
    ("deps.edn", &["clojure", "-P"]),
    ("project.clj", &["lein", "deps"]),
    // Julia
    ("Project.toml", &["julia", "--project", "-e", "using Pkg; Pkg.instantiate()"]),
    // Crystal
    ("shard.yml", &["shards", "install"]),
];

/// Update git before the rest of the bootstrap (defaults to `git fetch`).
pub fn git_update(worktree: &Path, command: Option<&[String]>) -> Result<()> {
    match command {
        Some(argv) => run_command(worktree, argv),
        None => {
            let owned: Vec<String> = DEFAULT_GIT_UPDATE.iter().map(|s| s.to_string()).collect();
            run_command(worktree, &owned)
        }
    }
}

/// Copy files (e.g. `.env`) from the source repo into the worktree.
/// Passing `None` copies the built-in default env-file list.
pub fn copy_files(source: &Path, worktree: &Path, files: Option<&[String]>) -> Result<()> {
    let default: Vec<String>;
    let files: &[String] = match files {
        Some(f) => f,
        None => {
            default = DEFAULT_COPY_FILES.iter().map(|s| s.to_string()).collect();
            &default
        }
    };
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
        if marker_matches(worktree, &rule.marker) {
            return run_command(worktree, &rule.command);
        }
    }
    for (marker, argv) in BUILTIN_RULES {
        if marker_matches(worktree, marker) {
            let owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
            return run_command(worktree, &owned);
        }
    }
    println!("[install] no matching install rule, skipping");
    Ok(())
}

/// Does a marker match in the worktree root? A `*.ext` marker matches any file
/// with that extension; anything else is an exact filename.
fn marker_matches(worktree: &Path, marker: &str) -> bool {
    if let Some(ext) = marker.strip_prefix("*.") {
        let suffix = format!(".{ext}");
        std::fs::read_dir(worktree).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|e| e.file_name().to_string_lossy().ends_with(&suffix))
        })
    } else {
        worktree.join(marker).exists()
    }
}

/// Run a list of hook commands in order. Any non-zero exit aborts.
pub fn run_hooks(worktree: &Path, hooks: &[CommandConfig]) -> Result<()> {
    for hook in hooks {
        run_command(worktree, &hook.command)?;
    }
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
