use crate::{git, go};
use anyhow::bail;

pub fn run(query: Option<&str>, force: bool) -> anyhow::Result<()> {
    let worktrees = git::worktree_list(None)?;
    let main_root = git::top_level(None)?;

    let selected = match query {
        Some(q) => go::select_by_query(&worktrees, q)?,
        None => go::select_by_fzf(&worktrees)?,
    };

    if selected.path == main_root {
        bail!("refusing to remove the main worktree");
    }

    git::worktree_remove(&selected.path, force, None)?;
    println!("removed {}", selected.path.display());
    Ok(())
}
