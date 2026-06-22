use crate::git;

pub fn run() -> anyhow::Result<()> {
    let worktrees = git::worktree_list(None)?;
    let current = git::top_level(None).ok();

    for wt in &worktrees {
        let mark = if Some(&wt.path) == current.as_ref() {
            "*"
        } else {
            " "
        };
        let branch_display = wt
            .branch
            .as_ref()
            .map(|b| b.strip_prefix("refs/heads/").unwrap_or(b.as_str()))
            .unwrap_or("(detached)");
        println!(
            "{} {}  {}",
            mark,
            wt.path.display(),
            branch_display
        );
    }
    Ok(())
}
