fn main() -> anyhow::Result<()> {
    let event_json = std::env::var("HERDR_PLUGIN_EVENT_JSON")?;

    let event: Event = serde_json::from_str(&event_json)?;

    println!("created: {}", event.data.worktree.path);
    println!("branch: {}", event.data.worktree.branch);

    if let Some(wt) = &event.data.workspace.worktree {
        println!("source repo: {}", wt.repo_root);
    }
}
