//! Undo the worktree herdr created, after its bootstrap failed.
//!
//! Every herdr plugin event is post-hoc: by the time this plugin is invoked the
//! worktree, its workspace and its pane all exist, and the exit code of the
//! event command is not a veto. "Don't create it on failure" is therefore not
//! something the plugin can do — the closest it can get is to ask herdr to take
//! it back down, which is what this module does when `[failure] action` is
//! `remove`.

use std::path::Path;
use std::process::Command;

use crate::herdr;

/// Ask herdr to remove the worktree, its workspace and its pane.
///
/// `--force` is not optional here: the worktree this runs against is by
/// definition half-bootstrapped, and herdr refuses a dirty one without it
/// (`dirty_worktree_requires_force`) — which would leave exactly the mess the
/// rollback was configured to prevent.
fn worktree_remove_command(workspace_id: &str) -> Vec<String> {
    ["worktree", "remove", "--workspace", workspace_id, "--force"]
        .iter()
        .map(|arg| arg.to_string())
        .collect()
}

/// Tear down the failed worktree, then the branch it left behind.
///
/// Never returns an error: the bootstrap has already failed and the process is
/// on its way to a non-zero exit, so a rollback that can't run is one more line
/// in the log rather than a second failure competing with the first.
pub fn remove_worktree(workspace_id: Option<&str>, branch: Option<&str>, source: Option<&Path>) {
    let Some(workspace_id) = workspace_id else {
        println!("[rollback] the event carried no workspace id, leaving the worktree in place");
        return;
    };

    println!("[rollback] removing workspace {workspace_id}");
    let argv = worktree_remove_command(workspace_id);
    match herdr::command().args(&argv).output() {
        Err(err) => println!("[rollback] could not run `herdr`: {err}"),
        Ok(output) if !output.status.success() => println!(
            "[rollback] herdr refused to remove the worktree: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Ok(_) => {
            println!("[rollback] removed the worktree, its workspace and its pane");
            delete_branch(branch, source);
        }
    }
}

/// Delete the branch the removed worktree was checked out on.
///
/// herdr removes the checkout but leaves the branch, so without this a rolled
/// back worktree still shows up in `git branch` and the next create collides
/// with its own leftovers.
///
/// Why not `-D`: `-d` refuses a branch carrying unmerged commits, which is the
/// one case where the branch is worth more than the tidiness — someone started
/// working in the pane during the bootstrap. Losing the branch there would be
/// silent and unrecoverable, so the refusal is the desired outcome.
fn delete_branch(branch: Option<&str>, source: Option<&Path>) {
    let (Some(branch), Some(source)) = (branch, source) else {
        return;
    };

    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["branch", "-d", branch])
        .output();

    match output {
        Err(err) => println!("[rollback] could not run `git`: {err}"),
        Ok(output) if !output.status.success() => println!(
            "[rollback] kept branch {branch}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Ok(_) => println!("[rollback] deleted branch {branch}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flag that makes the rollback work on the only kind of worktree it
    /// ever sees: one a failed bootstrap left dirty.
    #[test]
    fn the_removal_forces_past_a_dirty_worktree() {
        let argv = worktree_remove_command("wAK");
        assert_eq!(
            argv,
            ["worktree", "remove", "--workspace", "wAK", "--force"]
        );
    }
}
