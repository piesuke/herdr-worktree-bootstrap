//! `config::load` — which file wins, what a missing file means, and whether the
//! shipped examples still parse.

mod common;

use common::Dir;
use herdr_worktree_bootstrap::config::{self, Config};

/// The shipped examples, compiled in so a broken sample fails CI rather than a
/// user's first run.
const EXAMPLE_TOML: &str = include_str!("../examples/worktree-bootstrap.toml");
const EXAMPLE_YAML: &str = include_str!("../examples/worktree-bootstrap.yaml");

#[test]
fn a_repo_without_a_config_does_nothing() {
    let repo = Dir::new();
    let config = config::load(repo.path()).expect("a missing config is not an error");
    assert_eq!(config, Config::default());
}

#[test]
fn each_accepted_extension_is_loaded() {
    for name in [
        "worktree-bootstrap.toml",
        "worktree-bootstrap.yaml",
        "worktree-bootstrap.yml",
    ] {
        let repo = Dir::new();
        let text = if name.ends_with(".toml") {
            "[copy]\nenabled = true\n"
        } else {
            "copy:\n  enabled: true\n"
        };
        repo.write(&format!(".herdr/{name}"), text);

        let config = config::load(repo.path()).unwrap_or_else(|e| panic!("loading {name}: {e}"));
        assert!(config.copy.enabled, "{name} should have been loaded");
    }
}

/// The README warns never to commit both. When someone does anyway, `.toml`
/// wins and the YAML is ignored — pin that so the precedence can't drift.
#[test]
fn toml_wins_when_several_config_files_exist() {
    let repo = Dir::new();
    repo.write(".herdr/worktree-bootstrap.toml", "[copy]\nenabled = true\n");
    repo.write(
        ".herdr/worktree-bootstrap.yaml",
        "install:\n  enabled: true\n",
    );
    repo.write(".herdr/worktree-bootstrap.yml", "git:\n  update: true\n");

    let config = config::load(repo.path()).expect("config should load");
    assert!(config.copy.enabled, "the .toml should have been used");
    assert!(!config.install.enabled, "the .yaml should be ignored");
    assert!(!config.git.update, "the .yml should be ignored");
}

#[test]
fn yaml_wins_over_yml() {
    let repo = Dir::new();
    repo.write(".herdr/worktree-bootstrap.yaml", "copy:\n  enabled: true\n");
    repo.write(
        ".herdr/worktree-bootstrap.yml",
        "install:\n  enabled: true\n",
    );

    let config = config::load(repo.path()).expect("config should load");
    assert!(config.copy.enabled);
    assert!(!config.install.enabled);
}

/// The README's central claim about the two formats: same schema, so the two
/// shipped examples must deserialize to exactly the same config.
#[test]
fn the_toml_and_yaml_examples_are_equivalent() {
    let from_toml = Dir::new();
    from_toml.write(".herdr/worktree-bootstrap.toml", EXAMPLE_TOML);

    let from_yaml = Dir::new();
    from_yaml.write(".herdr/worktree-bootstrap.yaml", EXAMPLE_YAML);

    let toml_config = config::load(from_toml.path()).expect("examples/*.toml should parse");
    let yaml_config = config::load(from_yaml.path()).expect("examples/*.yaml should parse");

    assert_eq!(toml_config, yaml_config);
}

#[test]
fn a_broken_config_reports_the_file_it_came_from() {
    let repo = Dir::new();
    repo.write(".herdr/worktree-bootstrap.toml", "[copy\nenabled = true\n");

    let err = config::load(repo.path()).expect_err("malformed TOML should abort");
    let chain = format!("{err:#}");
    assert!(
        chain.contains("worktree-bootstrap.toml"),
        "error should name the config file, got: {chain}"
    );
}

#[test]
fn a_typo_in_a_repos_config_aborts_the_load() {
    let repo = Dir::new();
    repo.write(
        ".herdr/worktree-bootstrap.toml",
        "[copy]\nenabled = true\npattern = [\".env\"]\n",
    );

    let err = config::load(repo.path()).expect_err("an unknown key should abort");
    let chain = format!("{err:#}");
    assert!(chain.contains("pattern"), "got: {chain}");
}
