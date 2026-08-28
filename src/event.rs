//! Event payload delivered via the `HERDR_PLUGIN_EVENT_JSON` env var.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Event {
    pub data: EventData,
}

#[derive(Deserialize)]
pub struct EventData {
    /// The worktree that was just created (destination).
    pub worktree: Worktree,
    /// The workspace the worktree belongs to.
    pub workspace: Workspace,
}

#[derive(Deserialize)]
pub struct Worktree {
    pub path: String,
    pub branch: String,
}

#[derive(Deserialize)]
pub struct Workspace {
    /// Source worktree/repo this workspace was derived from.
    pub worktree: Option<SourceWorktree>,
}

#[derive(Deserialize)]
pub struct SourceWorktree {
    pub repo_root: String,
}
