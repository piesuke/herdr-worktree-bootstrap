# WorktreeBootstrap

A [Herdr](https://github.com/) plugin that bootstraps a freshly created git
worktree: it copies gitignored files (like `.env`), installs dependencies, and
runs your own pre/post commands — automatically, on `worktree.created`.

The plugin itself is generic. **Each repository configures its own bootstrap**
via a committed `.herdr/bootstrap.toml`, so different projects can copy
different files, install with different tools, and run different hooks.

## How it works

```
Herdr (worktree.created)
  └─ ./target/release/herdr-worktree-init
       │  reads HERDR_PLUGIN_EVENT_JSON (worktree path, branch, source repo_root)
       │  loads <repo>/.herdr/bootstrap.toml
       │
       ├─ pre hooks      commands run before anything else
       ├─ copy           <repo>/<file>  ->  <worktree>/<file>
       ├─ install        detect package manager from lockfiles, install
       └─ post hooks     commands run after copy + install
```

Lifecycle order: **pre → copy → install → post**. Any non-zero exit aborts the
whole bootstrap (fail-fast). If a repo has no `.herdr/bootstrap.toml`, the
plugin does nothing.

## Setup

### 1. Build the plugin

```sh
cargo build --release
```

This produces `./target/release/herdr-worktree-init`, which
`herdr-plugin.toml` invokes on `worktree.created`.

### 2. Configure a repository

Add `.herdr/bootstrap.toml` to any repo you want bootstrapped. A minimal
example:

```toml
[copy]
enabled = true
files = [".env", ".env.local"]

[install]
enabled = true

[[hooks.post]]
command = ["direnv", "allow"]
```

See [`examples/bootstrap.toml`](examples/bootstrap.toml) for the full schema.

## Configuration reference

The config file lives at `.herdr/bootstrap.toml` in each repository.

### `[copy]` — copy files into the worktree

Copies files from the source repo into the new worktree. Useful for gitignored
files (env files, local secrets) that a fresh checkout won't have. Missing
source files are skipped, not errors.

```toml
[copy]
enabled = true
files = [".env", ".env.local", ".env.development.local"]
```

### `[install]` — install dependencies

Detects the package manager from lockfiles/manifests present in the worktree
and runs the matching install command. The first matching marker wins.

```toml
[install]
enabled = true
```

Built-in detection (checked in this order):

| Language      | Marker file           | Command                                 |
| ------------- | --------------------- | --------------------------------------- |
| JS/TS         | `bun.lockb`           | `bun install`                           |
| JS/TS         | `pnpm-lock.yaml`      | `pnpm install --frozen-lockfile`        |
| JS/TS         | `yarn.lock`           | `yarn install --frozen-lockfile`        |
| JS/TS         | `package-lock.json`   | `npm ci`                                |
| JS/TS         | `package.json`        | `npm install`                           |
| Deno          | `deno.lock`           | `deno install`                          |
| Rust          | `Cargo.toml`          | `cargo fetch`                           |
| Go            | `go.mod`              | `go mod download`                       |
| Python        | `uv.lock`             | `uv sync`                               |
| Python        | `poetry.lock`         | `poetry install`                        |
| Python        | `Pipfile.lock`        | `pipenv install --dev`                  |
| Python        | `requirements.txt`    | `pip install -r requirements.txt`       |
| Ruby          | `Gemfile.lock`        | `bundle install`                        |
| PHP           | `composer.json`       | `composer install`                      |
| Java/Kotlin   | `pom.xml`             | `mvn install -DskipTests`               |
| Java/Kotlin   | `build.gradle`        | `gradle build -x test`                  |
| Elixir        | `mix.exs`             | `mix deps.get`                          |
| Dart/Flutter  | `pubspec.yaml`        | `dart pub get`                          |

**Custom rules** are checked *before* the built-ins, so they can add a language
or override one:

```toml
[[install.rules]]
marker = "flake.nix"
command = ["nix", "develop", "--command", "true"]
```

### `[[hooks.pre]]` / `[[hooks.post]]` — arbitrary commands

Commands run inside the new worktree, in order. `pre` runs before copy/install;
`post` runs after.

```toml
[[hooks.pre]]
command = ["mise", "install"]

[[hooks.post]]
command = ["direnv", "allow"]
```

> **Not run through a shell.** Each command is exec'd directly, so `&&`, pipes,
> `$VARS`, redirects, and globs do **not** work. Wrap them yourself:
>
> ```toml
> [[hooks.post]]
> command = ["sh", "-c", "npm run codegen && npm run build"]
> ```
>
> The program is resolved via `PATH`; if it isn't found the bootstrap aborts.

## Project layout

```
├── Cargo.toml
├── herdr-plugin.toml        # plugin manifest (generic, no per-repo settings)
├── examples/
│   └── bootstrap.toml       # sample .herdr/bootstrap.toml for consumers
└── src/
    ├── main.rs              # entry point: parse event, orchestrate phases
    ├── event.rs             # HERDR_PLUGIN_EVENT_JSON types
    ├── config.rs            # .herdr/bootstrap.toml types + loading
    └── bootstrap.rs         # copy / install / hook execution
```

## Security note

Because hooks and install commands come from the repo's committed
`.herdr/bootstrap.toml`, **anyone who can push to a repo can run arbitrary
commands** when a worktree of it is created — the same trust model as
`.git/hooks` or CI config. Only enable automatic bootstrap for repositories you
trust.
