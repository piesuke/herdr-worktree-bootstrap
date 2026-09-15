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
       ├─ git update     bring git up to date (e.g. fetch) — optional
       ├─ pre hooks      commands run before copy/install
       ├─ copy           <repo>/<file>  ->  <worktree>/<file>
       ├─ install        detect package manager from lockfiles, install
       └─ post hooks     commands run after copy + install
```

Lifecycle order: **git update → pre → copy → install → post**. Any non-zero
exit aborts the whole bootstrap (fail-fast). If a repo has no config file, the
plugin does nothing.

### Config format: TOML or YAML

The config can be written as **either TOML or YAML** — same schema, pick
whichever you prefer. The plugin looks for these files in order and uses the
first that exists:

```
.herdr/bootstrap.toml   (checked first)
.herdr/bootstrap.yaml
.herdr/bootstrap.yml
```

The reference below shows TOML. See
[`examples/bootstrap.yaml`](examples/bootstrap.yaml) for the identical config in
YAML.

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

The config file lives at `.herdr/bootstrap.toml` (or `.yaml`/`.yml`) in each
repository.

### `[git]` — update git first

Runs before everything else. Handy so a new worktree starts from the latest
remote state.

```toml
[git]
update = true
# command = ["git", "fetch", "--all", "--prune"]   # default; override as needed
```

Set `update = true` to run the update; `command` overrides the default
`git fetch --all --prune` (e.g. `["git", "pull", "--ff-only"]`).

### `[copy]` — copy files into the worktree

Brings gitignored files (env files, local secrets) that a fresh checkout won't
have into the new worktree. There are two modes.

**Discovery mode (default).** Omit `files`, and the plugin recursively finds
**gitignored** files in the source repo whose name matches `patterns` and copies
each to the same relative path in the worktree:

```toml
[copy]
enabled = true
# patterns = [".env", ".env.*"]   # omit for these env defaults; `*` = any chars
```

This is the right model for monorepos, where env files live in subdirectories
(`apps/web/.env`, `packages/db/.env.local`). Two properties make it safe:

- **Recursive** — files at any depth are found, not just the repo root.
- **Only gitignored files** — "is this gitignored?" is answered by git itself
  (`git ls-files`), so nested `.gitignore`s, negations, and globs all work.
  Committed files like `.env.example` are **never** copied: they aren't
  gitignored, and the fresh checkout already has them. Wholly-ignored
  directories (e.g. `node_modules/`) are skipped, not walked into.

**Explicit mode.** Set `files` to a list of relative paths to copy exactly those
instead (missing ones are skipped, not errors); discovery is then disabled:

```toml
[copy]
enabled = true
files = [".env", "apps/web/.env.local"]
```

### `[install]` — install dependencies

Detects the package manager from lockfiles/manifests present in the worktree
and runs the matching install command. The first matching marker wins; a
`*.ext` marker matches any file with that extension.

```toml
[install]
enabled = true
```

Built-in detection covers the major languages (checked in this order):

| Language      | Marker file           | Command                                 |
| ------------- | --------------------- | --------------------------------------- |
| JS/TS         | `bun.lockb`           | `bun install`                           |
| JS/TS         | `pnpm-lock.yaml`      | `pnpm install --frozen-lockfile`        |
| JS/TS         | `yarn.lock`           | `yarn install --frozen-lockfile`        |
| JS/TS         | `package-lock.json`   | `npm ci`                                |
| Deno          | `deno.lock`           | `deno install`                          |
| JS/TS         | `package.json`        | `npm install`                           |
| Rust          | `Cargo.toml`          | `cargo fetch`                           |
| Go            | `go.mod`              | `go mod download`                       |
| Python        | `uv.lock`             | `uv sync`                               |
| Python        | `poetry.lock`         | `poetry install`                        |
| Python        | `Pipfile.lock`        | `pipenv install --dev`                  |
| Python        | `requirements.txt`    | `pip install -r requirements.txt`       |
| Ruby          | `Gemfile`             | `bundle install`                        |
| PHP           | `composer.json`       | `composer install`                      |
| Java          | `pom.xml`             | `mvn install -DskipTests`               |
| Kotlin/Gradle | `build.gradle.kts`    | `gradle build -x test`                  |
| Java/Gradle   | `build.gradle`        | `gradle build -x test`                  |
| Scala         | `build.sbt`           | `sbt update`                            |
| C#/.NET       | `*.sln`               | `dotnet restore`                        |
| C#/.NET       | `*.csproj`            | `dotnet restore`                        |
| C/C++         | `vcpkg.json`          | `vcpkg install`                         |
| C/C++         | `conanfile.txt`       | `conan install .`                       |
| C/C++         | `conanfile.py`        | `conan install .`                       |
| Swift         | `Package.swift`       | `swift package resolve`                 |
| Obj-C/Swift   | `Podfile`             | `pod install`                           |
| Dart/Flutter  | `pubspec.yaml`        | `dart pub get`                          |
| Elixir        | `mix.exs`             | `mix deps.get`                          |
| Erlang        | `rebar.config`        | `rebar3 get-deps`                       |
| Haskell       | `stack.yaml`          | `stack build --only-dependencies`       |
| Haskell       | `cabal.project`       | `cabal build --only-dependencies`       |
| R             | `renv.lock`           | `Rscript -e 'renv::restore(...)'`       |
| Perl          | `cpanfile`            | `cpanm --installdeps .`                 |
| Clojure       | `deps.edn`            | `clojure -P`                            |
| Clojure       | `project.clj`         | `lein deps`                             |
| Julia         | `Project.toml`        | `julia --project -e 'Pkg.instantiate()'`|
| Crystal       | `shard.yml`           | `shards install`                        |

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
