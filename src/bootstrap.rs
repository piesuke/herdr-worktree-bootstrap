//! The three bootstrap phases: copy, install, run.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::{CommandConfig, InstallRule};

/// Default git update command when `[git] update = true` and no override given.
const DEFAULT_GIT_UPDATE: &[&str] = &["git", "fetch", "--all", "--prune"];

/// Default filename globs for recursive discovery when `[copy]` is enabled but
/// neither `files` nor `patterns` is set. Matches `.env` and `.env.<anything>`
/// (`.env.local`, `.env.production.local`, …) at any depth in the repo.
const DEFAULT_ENV_PATTERNS: &[&str] = &[".env", ".env.*"];

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

/// Copy an explicit list of relative paths from the source repo into the
/// worktree. Missing source files are skipped, not errors.
pub fn copy_files(source: &Path, worktree: &Path, files: &[String]) -> Result<()> {
    for file in files {
        let src = source.join(file);
        if !src.exists() {
            println!("[copy] skip (missing in source): {file}");
            continue;
        }
        copy_into_worktree(&src, worktree, Path::new(file))?;
        println!("[copy] {file}");
    }
    Ok(())
}

/// Recursively discover **gitignored** files in the source repo whose basename
/// matches one of `patterns` (default: env files) and copy each to the same
/// relative path in the worktree. This is the right model for env files in a
/// monorepo: they live in subdirectories and are gitignored, so a fresh
/// checkout won't have them — while committed files like `.env.example` are
/// left alone because they aren't gitignored.
///
/// "Which files are gitignored" is answered by git itself (`git ls-files`), so
/// nested `.gitignore` files, negations, and globs are all honored correctly.
pub fn copy_gitignored(source: &Path, worktree: &Path, patterns: Option<&[String]>) -> Result<()> {
    let default: Vec<String>;
    let patterns: &[String] = match patterns {
        Some(p) => p,
        None => {
            default = DEFAULT_ENV_PATTERNS.iter().map(|s| s.to_string()).collect();
            &default
        }
    };

    // `--others --ignored --exclude-standard` lists working-tree files that git
    // ignores; `--directory` collapses wholly-ignored dirs (e.g. node_modules/)
    // to a single entry so we don't walk into them. `-z` is NUL-delimited to
    // survive odd filenames.
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args([
            "ls-files",
            "-z",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
            "--full-name",
        ])
        .output()
        .context("running `git ls-files` to discover gitignored files")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "[copy] skip discovery: `git ls-files` failed ({})",
            stderr.trim()
        );
        return Ok(());
    }

    let mut copied = 0usize;
    for entry in output.stdout.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        let rel = String::from_utf8_lossy(entry);
        // A trailing slash means a collapsed ignored directory — skip it.
        if rel.ends_with('/') {
            continue;
        }
        let name = rel.rsplit('/').next().unwrap_or(&rel);
        if !patterns.iter().any(|pat| glob_match(pat, name)) {
            continue;
        }

        let src = source.join(&*rel);
        copy_into_worktree(&src, worktree, Path::new(&*rel))?;
        println!("[copy] {rel}");
        copied += 1;
    }

    if copied == 0 {
        println!("[copy] no gitignored files matched {patterns:?}");
    }
    Ok(())
}

/// Copy `src` to `worktree/rel`, creating parent directories as needed.
fn copy_into_worktree(src: &Path, worktree: &Path, rel: &Path) -> Result<()> {
    let dst = worktree.join(rel);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::copy(src, &dst)
        .with_context(|| format!("copying {} -> {}", src.display(), dst.display()))?;
    Ok(())
}

/// Glob match against a filename. Supports `*` (any sequence, including empty);
/// no `?` or character classes. Used to match discovered basenames.
fn glob_match(pattern: &str, name: &str) -> bool {
    fn helper(pat: &[u8], name: &[u8]) -> bool {
        match pat.split_first() {
            None => name.is_empty(),
            Some((b'*', rest)) => (0..=name.len()).any(|i| helper(rest, &name[i..])),
            Some((c, rest)) => name.first() == Some(c) && helper(rest, &name[1..]),
        }
    }
    helper(pattern.as_bytes(), name.as_bytes())
}

/// Install dependencies inside the worktree.
///
/// Detection runs independently in each of `dirs` (relative to the worktree
/// root, defaulting to the root itself), so a polyglot monorepo can install a
/// node app and a go service in one bootstrap. Within a directory the first
/// matching marker wins and exactly one command runs there.
///
/// A listed directory that doesn't exist aborts: `dirs` describes committed
/// repo structure, so a missing one means the config is wrong, and silently
/// installing nothing is the failure mode this option exists to fix.
pub fn install_deps(worktree: &Path, rules: &[InstallRule], dirs: Option<&[String]>) -> Result<()> {
    let default: Vec<String>;
    let dirs: &[String] = match dirs {
        Some(dirs) => dirs,
        None => {
            default = vec![".".to_string()];
            &default
        }
    };

    // Validate every directory before installing anything: a typo in the last
    // entry should not leave the earlier packages half-installed.
    for dir in dirs {
        if !worktree.join(dir).is_dir() {
            bail!("install dir `{dir}` does not exist in the worktree");
        }
    }

    for dir in dirs {
        install_in_dir(&worktree.join(dir), dir, rules)?;
    }
    Ok(())
}

/// Detect and install in a single directory. User-defined `rules` are checked
/// first (so they can add languages or override a built-in), then the built-in
/// table.
fn install_in_dir(base: &Path, label: &str, rules: &[InstallRule]) -> Result<()> {
    for rule in rules {
        if marker_matches(base, &rule.marker) {
            return run_command(base, &rule.command);
        }
    }
    for (marker, argv) in BUILTIN_RULES {
        if marker_matches(base, marker) {
            let owned: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
            return run_command(base, &owned);
        }
    }
    println!("[install] no matching install rule in {label}, skipping");
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

/// Run a list of hook commands in order. Any non-zero exit aborts. A hook's
/// `dir` selects a subdirectory of the worktree to run in (default: the root).
pub fn run_hooks(worktree: &Path, hooks: &[CommandConfig]) -> Result<()> {
    for hook in hooks {
        let cwd = match &hook.dir {
            Some(dir) => worktree.join(dir),
            None => worktree.to_path_buf(),
        };
        run_command(&cwd, &hook.command)?;
    }
    Ok(())
}

/// Run one command in `cwd`. Non-zero exit aborts (returns Err).
pub fn run_command(cwd: &Path, argv: &[String]) -> Result<()> {
    let Some((program, rest)) = argv.split_first() else {
        bail!("empty command in config");
    };
    // Checked up front: a bad `dir` would otherwise surface as a spawn failure
    // indistinguishable from "the program isn't installed".
    if !cwd.is_dir() {
        bail!(
            "working directory does not exist: {} (running `{}`)",
            cwd.display(),
            argv.join(" ")
        );
    }
    println!("[run] {}", argv.join(" "));

    let status = Command::new(program)
        .args(rest)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("spawning `{program}`"))?;

    if !status.success() {
        bail!("`{}` exited with {}", argv.join(" "), status);
    }
    Ok(())
}
