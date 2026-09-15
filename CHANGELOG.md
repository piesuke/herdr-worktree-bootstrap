# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Because the config schema uses `deny_unknown_fields`, **removing or renaming a
config key is breaking** — an existing repo's committed
`.herdr/worktree-bootstrap.toml` stops parsing and the bootstrap aborts. Such
changes are called out under **Changed** or **Removed**, never **Added**.

## [Unreleased]

### Added

- Copy phase: brings gitignored files (`.env` and friends) from the source repo
  into the new worktree. Discovery mode asks git itself which files are ignored,
  recursively, so nested `.gitignore`s and negations work and committed files
  like `.env.example` are never copied. Explicit mode (`files`) copies an exact
  list instead.
- Install phase: detects the package manager from a marker file and runs the
  matching install command — 37 built-in rules, plus per-repo `[[install.rules]]`
  that are checked first. `dirs` runs detection independently in each listed
  package of a monorepo.
- `[[hooks.pre]]` / `[[hooks.post]]`: arbitrary commands, exec'd directly (not
  through a shell), each optionally scoped to a subdirectory via `dir`.
- `[git]`: an update command (default `git fetch --all --prune`) run before
  everything else.
- Config in TOML or YAML, read from `.herdr/worktree-bootstrap.{toml,yaml,yml}`
  in the repo being bootstrapped. Unknown keys are an error rather than a silent
  default.
- Fail-fast: the first non-zero exit aborts the remaining phases.
- `[notify]`: a herdr toast when the bootstrap finishes, listing what each phase
  did, or the command that failed. Unlike every other section it defaults to on
  (`when = "always"`, also `"failure"` and `"never"`), because herdr captures the
  plugin's stdout rather than printing it — without a toast a run is invisible
  unless you go and read `herdr plugin log list`. Requires `[ui.toast] delivery`
  in herdr's own config; the plugin log says so when herdr suppresses a toast.

### Changed

- Replaced the unmaintained `serde_yaml` 0.9 with `serde_yaml_ng` 0.10. No
  behaviour change — the schema, the error messages, and unknown-key rejection
  are identical.

[Unreleased]: https://github.com/piesuke/herdr-worktree-bootstrap/commits/main
