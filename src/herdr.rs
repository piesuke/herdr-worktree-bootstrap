//! Where the `herdr` binary is.
//!
//! herdr sets `HERDR_BIN_PATH` on every plugin process, naming the executable
//! that launched it, and asks plugins to call back through it rather than
//! through `PATH`.
//!
//! Why not just `PATH`: the herdr a user is actually running is not necessarily
//! the one `PATH` resolves to — installed outside `PATH` entirely, or two builds
//! side by side. A toast, a failure pane or a rollback that went silently to the
//! wrong instance is worse than one that visibly did not run.

use std::ffi::OsString;
use std::process::Command;

/// Env var herdr sets on plugin processes, holding the path to itself.
const BIN_PATH_VAR: &str = "HERDR_BIN_PATH";

/// A [`Command`] for the herdr that launched this plugin.
pub fn command() -> Command {
    Command::new(binary_path(std::env::var_os(BIN_PATH_VAR)))
}

/// Resolve the binary to invoke from what the environment offered.
///
/// Split out from [`command`] so the fallback is testable without mutating the
/// process environment, which is `unsafe` in edition 2024 and races the other
/// tests in this binary regardless.
fn binary_path(configured: Option<OsString>) -> OsString {
    configured
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| OsString::from("herdr"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_env_var_names_the_binary_to_invoke() {
        assert_eq!(
            binary_path(Some(OsString::from("/opt/herdr/bin/herdr"))),
            OsString::from("/opt/herdr/bin/herdr")
        );
    }

    #[test]
    fn a_plugin_run_outside_herdr_falls_back_to_the_path_lookup() {
        assert_eq!(binary_path(None), OsString::from("herdr"));
    }

    #[test]
    fn an_empty_env_var_falls_back_rather_than_running_the_current_directory() {
        assert_eq!(binary_path(Some(OsString::new())), OsString::from("herdr"));
    }
}
