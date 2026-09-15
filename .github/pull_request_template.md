<!--
Why this change, not what it does — the diff already says what.
See CONTRIBUTING.md.
-->

## Why

## Checklist

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-features --locked`

If it applies:
- [ ] **Config schema changed** — both `examples/worktree-bootstrap.toml` and
      `.yaml` updated, and the breaking-vs-additive nature called out above
      (`deny_unknown_fields` makes removals and renames breaking)
