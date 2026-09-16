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
    pub branch: Option<String>,
}

#[derive(Deserialize)]
pub struct Workspace {
    /// herdr's handle for this workspace (e.g. `wAK`), and the only thing
    /// `herdr worktree remove` will accept to undo the creation.
    ///
    /// Optional even though herdr always sends it: a rollback that can't find
    /// an id should say so and leave the worktree alone, not abort the parse
    /// and take every successful bootstrap down with it.
    pub workspace_id: Option<String>,
    /// Source worktree/repo this workspace was derived from.
    pub worktree: Option<SourceWorktree>,
}

#[derive(Deserialize)]
pub struct SourceWorktree {
    pub repo_root: String,
}
