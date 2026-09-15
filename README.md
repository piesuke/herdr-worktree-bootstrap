# WorktreeBootstrap

A [Herdr](https://github.com/herdrdev/herdr) plugin that bootstraps a freshly created git
worktree: it copies gitignored files (like `.env`), installs dependencies, and
runs your own pre/post commands — automatically, on `worktree.created`.

The plugin itself is generic. **Each repository configures its own bootstrap**
via a committed `.herdr/worktree-bootstrap.toml`, so different projects can copy
different files, install with different tools, and run different hooks.

## How it works

```
Herdr (worktree.created)
  └─ ./target/release/herdr-worktree-init
       │  reads HERDR_PLUGIN_EVENT_JSON (worktree path, branch, source repo_root)
       │  loads <repo>/.herdr/worktree-bootstrap.toml
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
.herdr/worktree-bootstrap.toml   (checked first)
.herdr/worktree-bootstrap.yaml
.herdr/worktree-bootstrap.yml
```

The reference below shows TOML. See
[`examples/worktree-bootstrap.yaml`](examples/worktree-bootstrap.yaml) for the
identical config in YAML.

**Prefer TOML**, including in non-Rust repos. The schemas are identical, but
several fields in this one take values starting with `*` (`patterns`, and
`*.ext` install markers), and in YAML a leading `*` is alias syntax — an
unquoted `marker: *.csproj` is a parse error. TOML also reports unknown-key
errors with the offending line. Never commit both files: `.toml` wins and the
other is silently ignored.

**Unknown keys are an error.** A typo like `pattern` instead of `patterns`
aborts the bootstrap with a message naming the offending key and the valid ones,
rather than silently falling back to the default — a config that looks right but
does nothing is the most expensive failure mode here.

## Installing the plugin

Requires **herdr 0.7.0+** (`min_herdr_version` in the manifest) and a Rust
toolchain to build the binary. Linux and macOS only.

### Install from GitHub

```sh
herdr plugin install piesuke/herdr-worktree-bootstrap
```

### Or link a local checkout (for development)

```sh
git clone git@github.com:piesuke/herdr-worktree-bootstrap.git
herdr plugin link ./herdr-worktree-bootstrap        # add --disabled to link without enabling
```

Either way herdr registers the plugin under the id
**`piesuke.herdr.worktree.bootstrap`** and records it in
`~/.config/herdr/plugins.json`.

### Build the binary

The manifest declares the build steps (`cargo fetch`, then
`cargo build --release`), which produce `./target/release/herdr-worktree-init`
— the binary `[[events]]` invokes. If that file doesn't exist after installing,
run the build yourself from the plugin root:

```sh
cargo build --release
```

### Verify

```sh
herdr plugin list
```

The entry should show `enabled: true`. Editing `herdr-plugin.toml` afterwards
does **not** require re-linking — herdr re-reads the manifest from
`manifest_path` on its own.

## Using it

Once installed, the plugin is entirely passive: it runs whenever herdr creates a
worktree, for every repo. What it *does* is decided per repository.

### 1. Add a config to a repo

Nothing happens until a repo has one. Commit `.herdr/worktree-bootstrap.toml` to each
repo you want bootstrapped:

```toml
[copy]
enabled = true
files = [".env", ".env.local"]

[install]
enabled = true

[[hooks.post]]
command = ["direnv", "allow"]
```

See [`examples/worktree-bootstrap.toml`](examples/worktree-bootstrap.toml) for the full schema and
the [configuration reference](#configuration-reference) below for each section.

### 2. Create a worktree

Create a worktree of that repo in herdr as usual. The plugin fires on
`worktree.created` and runs the phases in order.

> There is currently **no way to trigger a run manually** — creating a worktree
> is the only entry point. Iterating on a config means creating (and deleting) a
> throwaway worktree.

### 3. Read the logs

The plugin's stdout is captured by herdr, not printed to your terminal. To see
what a run did:

```sh
herdr plugin log list --plugin piesuke.herdr.worktree.bootstrap --limit 5
```

Each entry carries the `exit_code`, `status`, and full `stdout`/`stderr`. Useful
things you'll see there:

| Output | Meaning |
| ------ | ------- |
| `no .herdr/worktree-bootstrap.{toml,yaml,yml} in <repo>, nothing to do` | The repo has no config — step 1 was skipped |
| `[copy] no gitignored files matched [...]` | Discovery ran but found nothing — check the files are actually gitignored |
| `[install] no matching install rule, skipping` | No known marker file in the worktree root |
| `unknown field ...` | A typo in the config; the bootstrap aborted |

### Managing the plugin

```sh
herdr plugin disable piesuke.herdr.worktree.bootstrap   # stop it firing, keep it registered
herdr plugin enable  piesuke.herdr.worktree.bootstrap
herdr plugin unlink  piesuke.herdr.worktree.bootstrap   # remove a linked local checkout
herdr plugin uninstall piesuke.herdr.worktree.bootstrap # remove an installed copy
```

## Configuration reference

The config file lives at `.herdr/worktree-bootstrap.toml` (or `.yaml`/`.yml`) in each
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

**Detection is not recursive.** By default only the worktree root is examined,
and exactly one install command runs. For a monorepo, list the packages to
install in `dirs` — detection then runs independently in each, so several
install commands can run:

```toml
[install]
enabled = true
dirs = ["apps/web", "services/api", "ml"]   # relative to the worktree root
```

A JS monorepo with a root `package.json` and lockfile needs no `dirs`: the root
install already handles workspaces. `dirs` is for the polyglot case — a node app
next to a go service next to a python package — which would otherwise install
nothing at all.

Paths are not globbed (`apps/*` won't expand), and a listed directory that
doesn't exist aborts the bootstrap before anything runs, rather than being
skipped — a typo there means the config is wrong, and installing nothing
silently is exactly what this option exists to prevent.

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
`post` runs after. Add `dir` to run a hook in one package of a monorepo instead
of at the root:

```toml
[[hooks.pre]]
command = ["mise", "install"]

[[hooks.post]]
command = ["direnv", "allow"]

[[hooks.post]]
command = ["npm", "run", "codegen"]
dir = "apps/web"                      # relative to the worktree root
```

A `dir` that doesn't exist aborts with an explicit error, rather than surfacing
as a confusing "failed to spawn" for the program.

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
│   ├── worktree-bootstrap.toml   # sample config for consumers to copy
│   └── worktree-bootstrap.yaml   # the same config in YAML
├── src/
│   ├── main.rs              # binary: parse the event, then hand off to run()
│   ├── lib.rs               # run(): the bootstrap lifecycle
│   ├── event.rs             # HERDR_PLUGIN_EVENT_JSON types
│   ├── config.rs            # .herdr/worktree-bootstrap.toml types + loading
│   └── bootstrap.rs         # copy / install / hook execution
└── tests/
    ├── common/mod.rs        # temp dirs and throwaway git repos
    ├── config_load.rs       # which config file wins; the examples still parse
    ├── copy.rs              # discovery against real git repositories
    └── lifecycle.rs         # phase order and fail-fast
```

The crate is a library plus a thin binary. Everything that decides *what
happens* lives in the library, so the lifecycle can be tested without herdr in
the loop; `main.rs` only reads `HERDR_PLUGIN_EVENT_JSON`, loads the repo's
config, and calls `run()`.

## Development

```sh
cargo test           # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Unit tests live beside the code they cover, in `#[cfg(test)] mod tests` at the
bottom of each module — that's what gives them access to the private pieces
worth pinning, like the glob matcher and the install-detection table.
Integration tests in `tests/` drive the public API instead, and they don't mock:
the copy tests create real git repositories and the lifecycle tests run real
shell hooks, because delegating to `git ls-files` and exec'ing commands *is* the
behaviour under test.

**Linux and macOS only.** The integration tests shell out to `sh`, and the
manifest declares those two platforms.

## Security note

Because hooks and install commands come from the repo's committed
`.herdr/worktree-bootstrap.toml`, **anyone who can push to a repo can run arbitrary
commands** when a worktree of it is created — the same trust model as
`.git/hooks` or CI config. Only enable automatic bootstrap for repositories you
trust.
